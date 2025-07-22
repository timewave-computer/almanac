// SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

interface IProcessor {
    function processMessage(
        bytes32 messageId,
        uint256 sourceChain,
        address sender,
        bytes calldata payload
    ) external returns (bool);
}

/**
 * @title UniversalGateway
 * @dev Simple gateway contract for cross-chain message sending and receiving
 */
contract UniversalGateway {
    address public owner;
    
    // The processor contract that handles messages
    address public processor;
    
    // The relayer address that can submit inbound messages
    address public relayer;
    
    // Message counter for outbound messages
    uint256 private _messageCounter;
    
    // Message registry to prevent duplicate processing
    mapping(bytes32 => bool) public processedMessages;
    
    // Events
    event MessageSent(bytes32 indexed messageId, uint256 targetChain, address recipient, bytes payload);
    event MessageDelivered(bytes32 indexed messageId, uint256 sourceChain, address sender, bytes payload, bool success);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event ProcessorUpdated(address previousProcessor, address newProcessor);
    event RelayerUpdated(address previousRelayer, address newRelayer);
    
    modifier onlyOwner() {
        require(msg.sender == owner, "UniversalGateway: caller is not the owner");
        _;
    }
    
    modifier onlyRelayer() {
        require(msg.sender == relayer, "UniversalGateway: caller is not the relayer");
        _;
    }
    
    constructor() {
        owner = msg.sender;
        _messageCounter = 0;
    }
    
    /**
     * @dev Sets the processor contract address
     * @param _processor The address of the processor contract
     */
    function setProcessor(address _processor) external onlyOwner {
        require(_processor != address(0), "UniversalGateway: processor cannot be zero address");
        address previousProcessor = processor;
        processor = _processor;
        emit ProcessorUpdated(previousProcessor, _processor);
    }
    
    /**
     * @dev Sets the relayer address
     * @param _relayer The address that can relay messages
     */
    function setRelayer(address _relayer) external onlyOwner {
        require(_relayer != address(0), "UniversalGateway: relayer cannot be zero address");
        address previousRelayer = relayer;
        relayer = _relayer;
        emit RelayerUpdated(previousRelayer, _relayer);
    }
    
    /**
     * @dev Sends a message to another chain
     * @param targetChain The ID of the target chain
     * @param recipient The address of the recipient on the target chain
     * @param payload The message data to be sent
     * @return messageId The unique identifier for the message
     */
    function sendMessage(
        uint256 targetChain,
        address recipient,
        bytes calldata payload
    ) external returns (bytes32) {
        _messageCounter++;
        bytes32 messageId = keccak256(abi.encodePacked(
            block.chainid,
            _messageCounter,
            msg.sender,
            targetChain,
            recipient,
            payload
        ));
        
        emit MessageSent(messageId, targetChain, recipient, payload);
        
        return messageId;
    }
    
    /**
     * @dev Delivers a message from another chain to the processor
     * @param messageId Unique identifier for the message
     * @param sourceChain ID of the chain where the message originated
     * @param sender Address that sent the message on the source chain
     * @param payload The message data to be processed
     */
    function deliverMessage(
        bytes32 messageId,
        uint256 sourceChain,
        address sender,
        bytes calldata payload
    ) external onlyRelayer returns (bool) {
        require(!processedMessages[messageId], "UniversalGateway: message already processed");
        require(processor != address(0), "UniversalGateway: processor not set");
        
        processedMessages[messageId] = true;
        
        bool success = IProcessor(processor).processMessage(
            messageId,
            sourceChain,
            sender,
            payload
        );
        
        emit MessageDelivered(messageId, sourceChain, sender, payload, success);
        
        return success;
    }
    
    /**
     * @dev Transfers ownership of the contract to a new address
     * @param newOwner Address to transfer ownership to
     */
    function transferOwnership(address newOwner) public onlyOwner {
        require(newOwner != address(0), "UniversalGateway: new owner is the zero address");
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }
} 