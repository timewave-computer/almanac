//! Generated client code for contract: cosmos1abc123456789

pub struct Cosmos1Abc123456789Client {
    contract_address: String,
}

impl Cosmos1Abc123456789Client {
    pub fn new(contract_address: String) -> Self {
        Self { contract_address }
    }
    
    pub fn contract_address(&self) -> &str {
        &self.contract_address
    }
}
