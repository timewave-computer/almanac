// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {Faucet} from "../contracts/solidity/Faucet.sol";

contract FaucetTest is Test {
    Faucet public faucet;
    address public owner;
    address public alice;
    address public bob;

    function setUp() public {
        owner = address(this);
        alice = makeAddr("alice");
        bob = makeAddr("bob");
        
        faucet = new Faucet();
    }
    
    function testName() public {
        assertEq(faucet.name(), "Faucet Token");
    }
    
    function testSymbol() public {
        assertEq(faucet.symbol(), "FCT");
    }
    
    function testMint() public {
        uint256 amount = 100 * 10**18;
        
        faucet.mint(alice, amount);
        assertEq(faucet.balanceOf(alice), amount);
        
        faucet.mint(bob, amount * 2);
        assertEq(faucet.balanceOf(bob), amount * 2);
    }
    
    function testTransfer() public {
        uint256 amount = 100 * 10**18;
        
        faucet.mint(alice, amount);
        assertEq(faucet.balanceOf(alice), amount);
        
        vm.prank(alice);
        faucet.transfer(bob, amount / 2);
        
        assertEq(faucet.balanceOf(alice), amount / 2);
        assertEq(faucet.balanceOf(bob), amount / 2);
    }
    
    function testOwnership() public {
        assertEq(faucet.owner(), owner);
        
        address newOwner = makeAddr("newOwner");
        faucet.transferOwnership(newOwner);
        
        assertEq(faucet.owner(), newOwner);
    }
} 