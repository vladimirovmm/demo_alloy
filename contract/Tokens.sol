// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "contract/console.sol";
import "contract/IERC20.sol";

contract DemoERC20 is IERC20 {
    string public constant name = "DemoERC20";
    string public constant symbol = "DERC";
    uint8 public constant decimals = 18;
    uint256 totalSupply_ = 10000 ether;

    mapping(address => uint256) balances;
    mapping(address => mapping (address => uint256)) allowed;

    constructor() {
	    balances[msg.sender] = totalSupply_;
    }

    function totalSupply() public override view returns (uint256) {
	    return totalSupply_;
    }

    function balanceOf(address tokenOwner) public override view returns (uint256) {
        return balances[tokenOwner];
    }

    function transfer(address receiver, uint256 numTokens) public override returns (bool) {
        emit console.Vote("fn transfer");
        emit console.VoteAdderss(msg.sender, "msg.sender");

        require(numTokens <= balances[msg.sender]);
        balances[msg.sender] = balances[msg.sender]-numTokens;
        balances[receiver] = balances[receiver]+numTokens;
        emit Transfer(msg.sender, receiver, numTokens);
        return true;
    }

    function approve(address delegate, uint256 numTokens) public override returns (bool) {
        
        emit console.Vote("fn approve");
        emit console.VoteAdderss(msg.sender, "msg.sender");
        emit console.VoteAdderss(delegate, "delegate");
        emit console.VoteNumber(numTokens, "numTokens");

        allowed[msg.sender][delegate] = numTokens;
        emit Approval(msg.sender, delegate, numTokens);
        return true;
    }

    function allowance(address owner, address delegate) public override view returns (uint) {
        return allowed[owner][delegate];
    }

    function transferFrom(address owner, address buyer, uint256 numTokens) public override returns (bool) {

        emit console.Vote("transferFrom");
        emit console.VoteAdderss(msg.sender, "msg.sender");
        emit console.VoteAdderss(owner, "owner");
        emit console.VoteAdderss(buyer, "buyer");
        emit console.VoteAdderss(address(this), "this");
        emit console.VoteNumber(numTokens, "numTokens");

        require(numTokens <= balances[owner]);
        require(numTokens <= allowed[owner][msg.sender]);

        balances[owner] = balances[owner]-numTokens;
        allowed[owner][msg.sender] = allowed[owner][msg.sender]-numTokens;
        balances[buyer] = balances[buyer]+numTokens;
        emit Transfer(owner, buyer, numTokens);
        return true;
    }
}

contract TestERC20 is IERC20 {
    string public constant name = "TestERC20";
    string public constant symbol = "TERC";
    uint8 public constant decimals = 8;
    uint256 totalSupply_ = 11000 ether;

    mapping(address => uint256) balances;
    mapping(address => mapping (address => uint256)) allowed;

    constructor() {
	    balances[msg.sender] = totalSupply_;
    }

    function totalSupply() public override view returns (uint256) {
	    return totalSupply_;
    }

    function balanceOf(address tokenOwner) public override view returns (uint256) {
        return balances[tokenOwner];
    }

    function transfer(address receiver, uint256 numTokens) public override returns (bool) {
        require(numTokens <= balances[msg.sender]);
        balances[msg.sender] = balances[msg.sender]-numTokens;
        balances[receiver] = balances[receiver]+numTokens;
        emit Transfer(msg.sender, receiver, numTokens);
        return true;
    }

    function approve(address delegate, uint256 numTokens) public override returns (bool) {
        
        emit console.Vote("fn approve");
        emit console.VoteAdderss(msg.sender, "msg.sender");
        emit console.VoteAdderss(delegate, "delegate");
        emit console.VoteNumber(numTokens, "numTokens");

        allowed[msg.sender][delegate] = numTokens;
        emit Approval(msg.sender, delegate, numTokens);
        return true;
    }

    function allowance(address owner, address delegate) public override view returns (uint) {
        return allowed[owner][delegate];
    }

    function transferFrom(address owner, address buyer, uint256 numTokens) public override returns (bool) {

        emit console.Vote("transferFrom");
        emit console.VoteAdderss(msg.sender, "msg.sender");
        emit console.VoteAdderss(owner, "owner");
        emit console.VoteAdderss(buyer, "buyer");
        emit console.VoteAdderss(address(this), "this");
        emit console.VoteNumber(numTokens, "numTokens");

        require(numTokens <= balances[owner]);
        require(numTokens <= allowed[owner][msg.sender]);

        balances[owner] = balances[owner]-numTokens;
        allowed[owner][msg.sender] = allowed[owner][msg.sender]-numTokens;
        balances[buyer] = balances[buyer]+numTokens;
        emit Transfer(owner, buyer, numTokens);
        return true;
    }
}

contract ExmERC20 is IERC20 {
    string public constant name = "ExmERC20";
    string public constant symbol = "EERC";
    uint8 public constant decimals = 6;
    uint256 totalSupply_ = 12000 ether;

    mapping(address => uint256) balances;
    mapping(address => mapping (address => uint256)) allowed;

    constructor() {
	    balances[msg.sender] = totalSupply_;
    }

    function totalSupply() public override view returns (uint256) {
	    return totalSupply_;
    }

    function balanceOf(address tokenOwner) public override view returns (uint256) {
        return balances[tokenOwner];
    }

    function transfer(address receiver, uint256 numTokens) public override returns (bool) {
        require(numTokens <= balances[msg.sender]);
        balances[msg.sender] = balances[msg.sender]-numTokens;
        balances[receiver] = balances[receiver]+numTokens;
        emit Transfer(msg.sender, receiver, numTokens);
        return true;
    }

    function approve(address delegate, uint256 numTokens) public override returns (bool) {
        
        emit console.Vote("fn approve");
        emit console.VoteAdderss(msg.sender, "msg.sender");
        emit console.VoteAdderss(delegate, "delegate");
        emit console.VoteNumber(numTokens, "numTokens");

        allowed[msg.sender][delegate] = numTokens;
        emit Approval(msg.sender, delegate, numTokens);
        return true;
    }

    function allowance(address owner, address delegate) public override view returns (uint) {
        return allowed[owner][delegate];
    }

    function transferFrom(address owner, address buyer, uint256 numTokens) public override returns (bool) {

        emit console.Vote("transferFrom");
        emit console.VoteAdderss(msg.sender, "msg.sender");
        emit console.VoteAdderss(owner, "owner");
        emit console.VoteAdderss(buyer, "buyer");
        emit console.VoteAdderss(address(this), "this");
        emit console.VoteNumber(numTokens, "numTokens");

        require(numTokens <= balances[owner]);
        require(numTokens <= allowed[owner][msg.sender]);

        balances[owner] = balances[owner]-numTokens;
        allowed[owner][msg.sender] = allowed[owner][msg.sender]-numTokens;
        balances[buyer] = balances[buyer]+numTokens;
        emit Transfer(owner, buyer, numTokens);
        return true;
    }
}
