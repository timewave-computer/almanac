// SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

/**
 * @title TestToken
 * @dev A simple ERC20 token for testing with minting capabilities
 */
contract TestToken {
    string public name;
    string public symbol;
    uint8 private _decimals;
    uint256 private _totalSupply;
    
    mapping(address => uint256) private _balances;
    mapping(address => mapping(address => uint256)) private _allowances;
    
    address public owner;
    
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    
    modifier onlyOwner() {
        require(msg.sender == owner, "TestToken: caller is not the owner");
        _;
    }
    
    /**
     * @dev Constructor that sets the name, symbol, and decimals for the token
     * @param name_ The name of the token
     * @param symbol_ The symbol of the token
     * @param decimals_ The number of decimals for the token (default is 18)
     */
    constructor(string memory name_, string memory symbol_, uint8 decimals_) {
        name = name_;
        symbol = symbol_;
        _decimals = decimals_;
        owner = msg.sender;
    }
    
    /**
     * @dev Returns the number of decimals used to get its user representation.
     */
    function decimals() public view returns (uint8) {
        return _decimals;
    }
    
    /**
     * @dev Returns the total supply of the token.
     */
    function totalSupply() public view returns (uint256) {
        return _totalSupply;
    }
    
    /**
     * @dev Returns the balance of the specified account.
     */
    function balanceOf(address account) public view returns (uint256) {
        return _balances[account];
    }
    
    /**
     * @dev Transfers tokens to the specified address.
     */
    function transfer(address to, uint256 amount) public returns (bool) {
        address from = msg.sender;
        require(from != address(0), "TestToken: transfer from the zero address");
        require(to != address(0), "TestToken: transfer to the zero address");
        require(_balances[from] >= amount, "TestToken: transfer amount exceeds balance");
        
        _balances[from] = _balances[from] - amount;
        _balances[to] = _balances[to] + amount;
        emit Transfer(from, to, amount);
        return true;
    }
    
    /**
     * @dev Returns the allowance granted by owner to spender.
     */
    function allowance(address owner_, address spender) public view returns (uint256) {
        return _allowances[owner_][spender];
    }
    
    /**
     * @dev Approves the spender to spend the specified amount of tokens.
     */
    function approve(address spender, uint256 amount) public returns (bool) {
        address owner_ = msg.sender;
        require(owner_ != address(0), "TestToken: approve from the zero address");
        require(spender != address(0), "TestToken: approve to the zero address");
        
        _allowances[owner_][spender] = amount;
        emit Approval(owner_, spender, amount);
        return true;
    }
    
    /**
     * @dev Transfers tokens from one address to another.
     */
    function transferFrom(address from, address to, uint256 amount) public returns (bool) {
        address spender = msg.sender;
        require(from != address(0), "TestToken: transfer from the zero address");
        require(to != address(0), "TestToken: transfer to the zero address");
        require(_balances[from] >= amount, "TestToken: transfer amount exceeds balance");
        
        uint256 currentAllowance = _allowances[from][spender];
        require(currentAllowance >= amount, "TestToken: transfer amount exceeds allowance");
        
        _allowances[from][spender] = currentAllowance - amount;
        _balances[from] = _balances[from] - amount;
        _balances[to] = _balances[to] + amount;
        emit Transfer(from, to, amount);
        return true;
    }
    
    /**
     * @dev Mints tokens to a specified address
     * @param to The address to mint tokens to
     * @param amount The amount of tokens to mint
     */
    function mint(address to, uint256 amount) public onlyOwner {
        require(to != address(0), "TestToken: mint to the zero address");
        
        _totalSupply += amount;
        _balances[to] += amount;
        emit Transfer(address(0), to, amount);
    }
    
    /**
     * @dev Burns tokens from a specified address
     * @param from The address to burn tokens from
     * @param amount The amount of tokens to burn
     */
    function burnFrom(address from, uint256 amount) public {
        uint256 currentAllowance = _allowances[from][msg.sender];
        require(currentAllowance >= amount, "TestToken: burn amount exceeds allowance");
        
        _allowances[from][msg.sender] = currentAllowance - amount;
        _burn(from, amount);
    }
    
    /**
     * @dev Burns tokens from the sender's address
     * @param amount The amount of tokens to burn
     */
    function burn(uint256 amount) public {
        _burn(msg.sender, amount);
    }
    
    /**
     * @dev Internal function to burn tokens
     */
    function _burn(address account, uint256 amount) internal {
        require(account != address(0), "TestToken: burn from the zero address");
        require(_balances[account] >= amount, "TestToken: burn amount exceeds balance");
        
        _balances[account] -= amount;
        _totalSupply -= amount;
        emit Transfer(account, address(0), amount);
    }
    
    /**
     * @dev Transfers ownership of the contract to a new account
     */
    function transferOwnership(address newOwner) public onlyOwner {
        require(newOwner != address(0), "TestToken: new owner is the zero address");
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }
} 