// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title UniversalGateway
 * @dev A simple gateway for cross-chain communication
 */
contract UniversalGateway {
    address public processor;
    address public relayer;
    
    struct Message {
        uint256 destinationChainId;
        address targetAddress;
        bytes payload;
        bool delivered;
    }
    
    mapping(bytes32 => Message) public messages;
    
    event MessageSent(bytes32 indexed messageId, uint256 destinationChainId, address targetAddress, bytes payload);
    event MessageDelivered(bytes32 indexed messageId, uint256 destinationChainId, address targetAddress, bytes payload);
    
    // Authorization modifiers
    modifier onlyRelayer() {
        require(msg.sender == relayer, "Only relayer can call this function");
        _;
    }
    
    modifier onlyProcessor() {
        require(msg.sender == processor, "Only processor can call this function");
        _;
    }
    
    /**
     * @dev Set the processor contract address
     * @param _processor The address of the processor contract
     */
    function setProcessor(address _processor) external {
        processor = _processor;
    }
    
    /**
     * @dev Set the relayer address
     * @param _relayer The address of the relayer
     */
    function setRelayer(address _relayer) external {
        relayer = _relayer;
    }
    
    /**
     * @dev Send a message to another chain
     * @param destinationChainId The ID of the destination chain
     * @param targetAddress The address that will receive the message on the destination chain
     * @param payload The message payload
     * @return messageId The ID of the message
     */
    function sendMessage(uint256 destinationChainId, address targetAddress, bytes calldata payload) external returns (bytes32) {
        bytes32 messageId = keccak256(abi.encodePacked(destinationChainId, targetAddress, payload, block.timestamp, msg.sender));
        
        messages[messageId] = Message({
            destinationChainId: destinationChainId,
            targetAddress: targetAddress,
            payload: payload,
            delivered: false
        });
        
        emit MessageSent(messageId, destinationChainId, targetAddress, payload);
        
        return messageId;
    }
    
    /**
     * @dev Deliver a message from another chain
     * @param messageId The ID of the message to deliver
     * @param sourceChainId The ID of the source chain
     * @param sender The address that sent the message on the source chain
     * @param payload The message payload
     */
    function deliverMessage(bytes32 messageId, uint256 sourceChainId, address sender, bytes calldata payload) external onlyRelayer {
        // In a real implementation, this would verify the message and call the target contract
        // For testing, we just mark the message as delivered
        
        Message storage message = messages[messageId];
        message.delivered = true;
        
        emit MessageDelivered(messageId, sourceChainId, sender, payload);
    }
} 