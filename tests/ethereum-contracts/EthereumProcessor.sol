// SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

/**
 * @title EthereumProcessor
 * @dev Simple processor contract for cross-chain message handling
 */
contract EthereumProcessor {
    address public owner;
    address public gateway;
    
    // Events
    event MessageReceived(bytes32 messageId, uint256 sourceChain, address sender, bytes payload);
    event MessageProcessed(bytes32 messageId, bool success);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event GatewayUpdated(address previousGateway, address newGateway);
    
    modifier onlyOwner() {
        require(msg.sender == owner, "EthereumProcessor: caller is not the owner");
        _;
    }
    
    modifier onlyGateway() {
        require(msg.sender == gateway, "EthereumProcessor: caller is not the gateway");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    /**
     * @dev Sets the gateway address that can send messages to this processor
     * @param _gateway The address of the gateway contract
     */
    function setGateway(address _gateway) external onlyOwner {
        require(_gateway != address(0), "EthereumProcessor: gateway cannot be zero address");
        address previousGateway = gateway;
        gateway = _gateway;
        emit GatewayUpdated(previousGateway, _gateway);
    }
    
    /**
     * @dev Processes a cross-chain message
     * @param messageId Unique identifier for the message
     * @param sourceChain ID of the chain where the message originated
     * @param sender Address that sent the message on the source chain
     * @param payload The message data to process
     */
    function processMessage(
        bytes32 messageId,
        uint256 sourceChain,
        address sender,
        bytes calldata payload
    ) external onlyGateway returns (bool) {
        emit MessageReceived(messageId, sourceChain, sender, payload);
        
        // In a real implementation, we would:
        // 1. Decode the payload
        // 2. Validate the message
        // 3. Execute the appropriate action based on the message content
        
        // For test purposes, we just emit an event and return success
        emit MessageProcessed(messageId, true);
        return true;
    }
    
    /**
     * @dev Transfers ownership of the contract to a new address
     * @param newOwner Address to transfer ownership to
     */
    function transferOwnership(address newOwner) public onlyOwner {
        require(newOwner != address(0), "EthereumProcessor: new owner is the zero address");
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }
} 