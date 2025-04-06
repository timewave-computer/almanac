// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title EthereumProcessor
 * @dev A simple processor for cross-chain communication
 */
contract EthereumProcessor {
    address public gateway;
    
    event MessageProcessed(bytes32 indexed messageId, address sender, bytes payload);
    
    // Authorization modifiers
    modifier onlyGateway() {
        require(msg.sender == gateway, "Only gateway can call this function");
        _;
    }
    
    /**
     * @dev Set the gateway contract address
     * @param _gateway The address of the gateway contract
     */
    function setGateway(address _gateway) external {
        gateway = _gateway;
    }
    
    /**
     * @dev Process a message from another chain
     * @param messageId The ID of the message to process
     * @param sender The address that sent the message on the source chain
     * @param payload The message payload
     */
    function processMessage(bytes32 messageId, address sender, bytes calldata payload) external onlyGateway {
        // In a real implementation, this would process the message and perform actions
        // For testing, we just emit an event
        
        emit MessageProcessed(messageId, sender, payload);
    }
} 