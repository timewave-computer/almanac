// SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

/**
 * @title BaseAccount
 * @dev Simple account contract that allows authorized addresses to control it
 */
contract BaseAccount {
    address public owner;
    
    // Mapping of authorized addresses
    mapping(address => bool) public authorized;
    
    // Events
    event Transfer(address indexed token, address indexed to, uint256 amount);
    event Execution(address indexed target, uint256 value, bytes data);
    event AuthorizationChanged(address indexed account, bool isAuthorized);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    
    modifier onlyOwner() {
        require(msg.sender == owner, "BaseAccount: caller is not the owner");
        _;
    }
    
    modifier onlyAuthorized() {
        require(msg.sender == owner || authorized[msg.sender], "BaseAccount: caller is not authorized");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    /**
     * @dev Authorizes or deauthorizes an address to control this account
     * @param account Address to change authorization for
     * @param isAuthorized Whether the address should be authorized
     */
    function authorize(address account, bool isAuthorized) external onlyOwner {
        authorized[account] = isAuthorized;
        emit AuthorizationChanged(account, isAuthorized);
    }
    
    /**
     * @dev Checks if an address is authorized
     * @param account Address to check
     * @return bool Whether the address is authorized
     */
    function isAuthorized(address account) external view returns (bool) {
        return account == owner || authorized[account];
    }
    
    /**
     * @dev Executes a call to another contract
     * @param target Address of the contract to call
     * @param data The calldata to send
     * @return success Whether the call succeeded
     * @return returnData The data returned by the call
     */
    function execute(address target, bytes calldata data) external onlyAuthorized returns (bool success, bytes memory returnData) {
        (success, returnData) = target.call(data);
        require(success, "BaseAccount: execution failed");
        emit Execution(target, 0, data);
        return (success, returnData);
    }
    
    /**
     * @dev Executes a call to another contract with a value
     * @param target Address of the contract to call
     * @param value Amount of ETH to send
     * @param data The calldata to send
     * @return success Whether the call succeeded
     * @return returnData The data returned by the call
     */
    function executeWithValue(address target, uint256 value, bytes calldata data) external onlyAuthorized returns (bool success, bytes memory returnData) {
        (success, returnData) = target.call{value: value}(data);
        require(success, "BaseAccount: execution failed");
        emit Execution(target, value, data);
        return (success, returnData);
    }
    
    /**
     * @dev Transfers ownership of the account to a new address
     * @param newOwner Address of the new owner
     */
    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "BaseAccount: new owner is the zero address");
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }
    
    /**
     * @dev Receive function to accept ETH
     */
    receive() external payable {}
} 