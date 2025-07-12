use crate::{Error, Result};
use bytes::{Bytes, BytesMut, BufMut};
use flate2::write::{GzEncoder, GzDecoder};
use flate2::Compression;
use serde::{Serialize, Deserialize};
use std::io::Write;
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};

/// Optimized serialization format options
#[derive(Debug, Clone, Copy)]
pub enum SerializationFormat {
    /// JSON format (human readable, larger)
    Json,
    /// MessagePack format (binary, smaller)
    MessagePack,
    /// Protocol Buffers (requires schema)
    Protobuf,
}

/// Compression options for serialized data
#[derive(Debug, Clone, Copy)]
pub enum CompressionType {
    /// No compression
    None,
    /// Gzip compression
    Gzip(CompressionLevel),
    /// LZ4 compression (faster but less compression)
    Lz4,
    /// Zstd compression (good balance)
    Zstd(i32),
}

#[derive(Debug, Clone, Copy)]
pub enum CompressionLevel {
    Fast,
    Default,
    Best,
}

impl From<CompressionLevel> for Compression {
    fn from(level: CompressionLevel) -> Self {
        match level {
            CompressionLevel::Fast => Compression::fast(),
            CompressionLevel::Default => Compression::default(),
            CompressionLevel::Best => Compression::best(),
        }
    }
}

/// Optimized serializer for distributed query data
pub struct OptimizedSerializer {
    format: SerializationFormat,
    compression: CompressionType,
    /// Buffer size for streaming operations
    buffer_size: usize,
}

impl OptimizedSerializer {
    pub fn new(format: SerializationFormat, compression: CompressionType) -> Self {
        Self {
            format,
            compression,
            buffer_size: 64 * 1024, // 64KB default buffer
        }
    }

    /// Serialize data to bytes with optional compression
    pub fn serialize<T: Serialize>(&self, data: &T) -> Result<Bytes> {
        // First serialize to the chosen format
        let serialized = match self.format {
            SerializationFormat::Json => {
                serde_json::to_vec(data)
                    .map_err(|e| Error::Serialization(e.to_string()))?
            }
            SerializationFormat::MessagePack => {
                rmp_serde::to_vec(data)
                    .map_err(|e| Error::Serialization(e.to_string()))?
            }
            SerializationFormat::Protobuf => {
                // Would require protobuf schema
                return Err(Error::Generic("Protobuf not implemented".to_string()));
            }
        };

        // Then apply compression if requested
        let compressed = match self.compression {
            CompressionType::None => serialized,
            CompressionType::Gzip(level) => {
                let mut encoder = GzEncoder::new(Vec::new(), level.into());
                encoder.write_all(&serialized)
                    .map_err(|e| Error::Serialization(e.to_string()))?;
                encoder.finish()
                    .map_err(|e| Error::Serialization(e.to_string()))?
            }
            CompressionType::Lz4 => {
                lz4_flex::compress_prepend_size(&serialized)
            }
            CompressionType::Zstd(level) => {
                zstd::encode_all(&serialized[..], level)
                    .map_err(|e| Error::Serialization(e.to_string()))?
            }
        };

        Ok(Bytes::from(compressed))
    }

    /// Deserialize data from bytes with optional decompression
    pub fn deserialize<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Result<T> {
        // First decompress if needed
        let decompressed = match self.compression {
            CompressionType::None => data.to_vec(),
            CompressionType::Gzip(_) => {
                let mut decoder = GzDecoder::new(Vec::new());
                decoder.write_all(data)
                    .map_err(|e| Error::Deserialization(e.to_string()))?;
                decoder.finish()
                    .map_err(|e| Error::Deserialization(e.to_string()))?
            }
            CompressionType::Lz4 => {
                lz4_flex::decompress_size_prepended(data)
                    .map_err(|e| Error::Deserialization(e.to_string()))?
            }
            CompressionType::Zstd(_) => {
                zstd::decode_all(data)
                    .map_err(|e| Error::Deserialization(e.to_string()))?
            }
        };

        // Then deserialize from the chosen format
        match self.format {
            SerializationFormat::Json => {
                serde_json::from_slice(&decompressed)
                    .map_err(|e| Error::Deserialization(e.to_string()))
            }
            SerializationFormat::MessagePack => {
                rmp_serde::from_slice(&decompressed)
                    .map_err(|e| Error::Deserialization(e.to_string()))
            }
            SerializationFormat::Protobuf => {
                Err(Error::Generic("Protobuf not implemented".to_string()))
            }
        }
    }

    /// Stream serialize data chunks for large datasets
    pub async fn serialize_stream<T, W>(
        &self,
        items: impl Iterator<Item = T>,
        writer: &mut W,
    ) -> Result<usize>
    where
        T: Serialize,
        W: AsyncWrite + Unpin,
    {
        let mut total_bytes = 0;
        let mut buffer = BytesMut::with_capacity(self.buffer_size);

        for item in items {
            let serialized = self.serialize(&item)?;
            
            // Write length prefix for framing
            buffer.put_u32(serialized.len() as u32);
            buffer.put(serialized);

            // Flush buffer if it's getting full
            if buffer.len() >= self.buffer_size {
                writer.write_all(&buffer).await
                    .map_err(|e| Error::Io(e.to_string()))?;
                total_bytes += buffer.len();
                buffer.clear();
            }
        }

        // Flush remaining buffer
        if !buffer.is_empty() {
            writer.write_all(&buffer).await
                .map_err(|e| Error::Io(e.to_string()))?;
            total_bytes += buffer.len();
        }

        writer.flush().await
            .map_err(|e| Error::Io(e.to_string()))?;

        Ok(total_bytes)
    }

    /// Stream deserialize data chunks
    pub async fn deserialize_stream<T, R>(
        &self,
        reader: &mut R,
    ) -> Result<Vec<T>>
    where
        T: for<'de> Deserialize<'de>,
        R: AsyncRead + Unpin,
    {
        let mut results = Vec::new();
        let mut length_buf = [0u8; 4];

        loop {
            // Read length prefix
            match reader.read_exact(&mut length_buf).await {
                Ok(_) => {},
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(Error::Io(e.to_string())),
            }

            let length = u32::from_be_bytes(length_buf) as usize;
            
            // Read data
            let mut data = vec![0u8; length];
            reader.read_exact(&mut data).await
                .map_err(|e| Error::Io(e.to_string()))?;

            // Deserialize
            let item: T = self.deserialize(&data)?;
            results.push(item);
        }

        Ok(results)
    }
}

/// Batch serializer for efficient bulk operations
pub struct BatchSerializer {
    serializer: OptimizedSerializer,
    batch_size: usize,
}

impl BatchSerializer {
    pub fn new(serializer: OptimizedSerializer, batch_size: usize) -> Self {
        Self {
            serializer,
            batch_size,
        }
    }

    /// Serialize items in batches for better compression ratios
    pub fn serialize_batch<T: Serialize>(&self, items: &[T]) -> Result<Vec<Bytes>> {
        let mut results = Vec::new();

        for chunk in items.chunks(self.batch_size) {
            let batch_data = BatchData {
                items: chunk,
                count: chunk.len(),
            };
            results.push(self.serializer.serialize(&batch_data)?);
        }

        Ok(results)
    }

    /// Deserialize batched data
    pub fn deserialize_batch<T>(&self, batches: &[Bytes]) -> Result<Vec<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut results = Vec::new();

        for batch_bytes in batches {
            let batch: BatchData<T> = self.serializer.deserialize(batch_bytes)?;
            results.extend(batch.items);
        }

        Ok(results)
    }
}

#[derive(Serialize, Deserialize)]
struct BatchData<T> {
    items: Vec<T>,
    count: usize,
}

/// Cached serializer that remembers frequently serialized objects
pub struct CachedSerializer {
    serializer: OptimizedSerializer,
    cache: lru::LruCache<u64, Bytes>,
}

impl CachedSerializer {
    pub fn new(serializer: OptimizedSerializer, cache_size: usize) -> Self {
        Self {
            serializer,
            cache: lru::LruCache::new(cache_size.try_into().unwrap()),
        }
    }

    /// Serialize with caching based on content hash
    pub fn serialize_cached<T: Serialize>(&mut self, data: &T) -> Result<Bytes> {
        // Calculate hash of the data
        let json = serde_json::to_string(data)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        let hash = xxhash_rust::xxh64::xxh64(json.as_bytes(), 0);

        // Check cache
        if let Some(cached) = self.cache.get(&hash) {
            return Ok(cached.clone());
        }

        // Serialize and cache
        let serialized = self.serializer.serialize(data)?;
        self.cache.put(hash, serialized.clone());

        Ok(serialized)
    }
}

/// Protocol for efficient node-to-node communication
pub struct WireProtocol {
    version: u8,
    serializer: OptimizedSerializer,
}

impl WireProtocol {
    pub fn new(serializer: OptimizedSerializer) -> Self {
        Self {
            version: 1,
            serializer,
        }
    }

    /// Encode a message with header for the wire
    pub fn encode_message<T: Serialize>(&self, message_type: MessageType, data: &T) -> Result<Bytes> {
        let payload = self.serializer.serialize(data)?;
        
        let mut buffer = BytesMut::new();
        
        // Protocol header
        buffer.put_u8(self.version);
        buffer.put_u8(message_type as u8);
        buffer.put_u32(payload.len() as u32);
        
        // Payload
        buffer.put(payload);
        
        Ok(buffer.freeze())
    }

    /// Decode a message from the wire
    pub fn decode_message<T>(&self, data: &[u8]) -> Result<(MessageType, T)>
    where
        T: for<'de> Deserialize<'de>,
    {
        if data.len() < 6 {
            return Err(Error::Generic("Invalid message format".to_string()));
        }

        let version = data[0];
        if version != self.version {
            return Err(Error::Generic(format!("Unsupported protocol version: {}", version)));
        }

        let message_type = MessageType::try_from(data[1])
            .map_err(|_| Error::Generic("Invalid message type".to_string()))?;

        let payload_len = u32::from_be_bytes([data[2], data[3], data[4], data[5]]) as usize;
        
        if data.len() < 6 + payload_len {
            return Err(Error::Generic("Incomplete message".to_string()));
        }

        let payload = &data[6..6 + payload_len];
        let decoded = self.serializer.deserialize(payload)?;

        Ok((message_type, decoded))
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum MessageType {
    QueryRequest = 1,
    QueryResponse = 2,
    StreamChunk = 3,
    Error = 4,
    Heartbeat = 5,
}

impl TryFrom<u8> for MessageType {
    type Error = ();

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            1 => Ok(MessageType::QueryRequest),
            2 => Ok(MessageType::QueryResponse),
            3 => Ok(MessageType::StreamChunk),
            4 => Ok(MessageType::Error),
            5 => Ok(MessageType::Heartbeat),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestData {
        id: u64,
        name: String,
        values: Vec<f64>,
    }

    #[test]
    fn test_json_serialization() {
        let serializer = OptimizedSerializer::new(
            SerializationFormat::Json,
            CompressionType::None,
        );

        let data = TestData {
            id: 42,
            name: "test".to_string(),
            values: vec![1.0, 2.0, 3.0],
        };

        let serialized = serializer.serialize(&data).unwrap();
        let deserialized: TestData = serializer.deserialize(&serialized).unwrap();

        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_messagepack_serialization() {
        let serializer = OptimizedSerializer::new(
            SerializationFormat::MessagePack,
            CompressionType::None,
        );

        let data = TestData {
            id: 42,
            name: "test".to_string(),
            values: vec![1.0, 2.0, 3.0],
        };

        let serialized = serializer.serialize(&data).unwrap();
        let deserialized: TestData = serializer.deserialize(&serialized).unwrap();

        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_gzip_compression() {
        let serializer = OptimizedSerializer::new(
            SerializationFormat::Json,
            CompressionType::Gzip(CompressionLevel::Default),
        );

        let data = TestData {
            id: 42,
            name: "test".to_string(),
            values: vec![1.0; 1000], // Repetitive data for good compression
        };

        let serialized = serializer.serialize(&data).unwrap();
        let deserialized: TestData = serializer.deserialize(&serialized).unwrap();

        assert_eq!(data.id, deserialized.id);
        assert_eq!(data.name, deserialized.name);
        assert_eq!(data.values.len(), deserialized.values.len());
    }

    #[test]
    fn test_wire_protocol() {
        let serializer = OptimizedSerializer::new(
            SerializationFormat::MessagePack,
            CompressionType::Lz4,
        );
        let protocol = WireProtocol::new(serializer);

        let data = TestData {
            id: 123,
            name: "protocol test".to_string(),
            values: vec![1.0, 2.0, 3.0],
        };

        let encoded = protocol.encode_message(MessageType::QueryResponse, &data).unwrap();
        let (msg_type, decoded): (MessageType, TestData) = protocol.decode_message(&encoded).unwrap();

        assert!(matches!(msg_type, MessageType::QueryResponse));
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_batch_serialization() {
        let serializer = OptimizedSerializer::new(
            SerializationFormat::Json,
            CompressionType::Zstd(3),
        );
        let batch_serializer = BatchSerializer::new(serializer, 10);

        let items: Vec<TestData> = (0..25).map(|i| TestData {
            id: i,
            name: format!("item_{}", i),
            values: vec![i as f64; 10],
        }).collect();

        let batches = batch_serializer.serialize_batch(&items).unwrap();
        assert_eq!(batches.len(), 3); // 25 items / 10 batch size = 3 batches

        let deserialized = batch_serializer.deserialize_batch(&batches).unwrap();
        assert_eq!(items.len(), deserialized.len());
    }
}