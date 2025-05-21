// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "contract/console.sol";

error InsFund(uint, uint, address);

interface IERC20 {
    function totalSupply() external view returns (uint256);
    function balanceOf(address account) external view returns (uint256);
    function allowance(address owner, address spender) external view returns (uint256);
    function transfer(address recipient, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
    function transferFrom(address sender, address recipient, uint256 amount) external returns (bool);
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
}



contract DemoERC20 is IERC20 {
    string public constant name = "DemoERC20";
    string public constant symbol = "ERC";
    uint8 public constant decimals = 18;
    uint256 totalSupply_ = 10 ether;
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
        
        emit console.VoteString("fn approve");
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

        emit console.VoteString("transferFrom");
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


// Deposite l1 => l2
// Withdraw l2 => l1
contract Bridge {
    address owner;

    event Send(address from, address to, uint value);
    event SendERC20(
        address token_address,
        address from, 
        address to, 
        uint value
    );


    mapping (address => uint) WithdrowRequestes;
    mapping (address =>  mapping (address => uint)) WithdrowRequestesErc20;

    constructor() {
        owner = msg.sender;
    }

    // (ETH) l1 => l2 
    function deposite(address receiver) payable public returns (bool) {
        emit console.VoteString("deposite");
        emit console.VoteAdderss(msg.sender, "sender");

        uint256 amount_deposit = msg.value;
        bool sent = payable(owner).send(amount_deposit);
        require(sent, "Failed to send ETH");
        emit Send(msg.sender, receiver, amount_deposit);
        return true;
    }

    // (ERC20) l1 => l2
    function deposite_erc20(address tokenContract, address receiver, uint amount_deposit ) public returns (bool){

        emit console.VoteString("deposite_erc20");
        emit console.VoteAdderss(msg.sender, "msg.sender");
        emit console.VoteAdderss(tokenContract, "tokenContract");
        emit console.VoteAdderss(receiver, "receiver");
        emit console.VoteAdderss(address(this), "this");
        emit console.VoteNumber(amount_deposit, "amount_deposit");
        
        IERC20 token = IERC20(tokenContract);

        uint balance = token.allowance(msg.sender, address(this));
        emit console.VoteNumber(balance, "token.allowance");
        require(balance >= amount_deposit ,"Approve the transfer");

        bool send = token.transferFrom(msg.sender, owner, amount_deposit);
        require(send, "Failed to send ");

        emit SendERC20(tokenContract, msg.sender, receiver, amount_deposit);
        return true;
    }


    // (ETH) L2 => L1
    // Owner: reserves coins for the client
    function request_withdraw(address to) payable public returns (bool){
        WithdrowRequestes[to] = WithdrowRequestes[to] + msg.value;
        return true;
    }

    // (ERC20) L2 => L1
    // Owner: reserves coins for the client
    function request_withdraw_erc20(address tokenContract, address to, uint amount) payable public returns (bool){

        emit console.VoteString("request_withdraw_erc20");
        emit console.VoteAdderss(msg.sender, "msg.sender");
        emit console.VoteAdderss(tokenContract, "tokenContract");
        emit console.VoteAdderss(to, "to");
        emit console.VoteNumber(amount, "amount");

        require(msg.sender == owner, "Access is denied");

        IERC20 token = IERC20(tokenContract);

        uint balance = token.allowance(owner, address(this));
        emit console.VoteNumber(balance, "token.allowance");

        require(balance >= amount ,"Approve the transfer");

        WithdrowRequestesErc20[to][tokenContract] = WithdrowRequestesErc20[to][tokenContract] + amount;
        
        return true;
    }

    function status_withdraw() public view returns (uint){
        return WithdrowRequestes[msg.sender];
    }

    function status_withdraw_erc20(address tokenContract) public view returns (uint){
        return WithdrowRequestesErc20[msg.sender][tokenContract];
    }

    // (ETH) Withdraw => Client
    // The client withdraws the coins to his account
    function withdraw() payable public returns (bool){
        require(WithdrowRequestes[msg.sender] > 0, "There are no funds for withdrawal");
        uint amount = WithdrowRequestes[msg.sender];
        WithdrowRequestes[msg.sender] = 0;
        payable(msg.sender).transfer(amount);

        return true;
    }

    // (ERC20) Withdraw => Client
    // The client withdraws the coins to his account
    function withdraw_erc20(address tokenContract) payable public returns (bool){
        
        emit console.VoteString("deposite_erc20"); 
        emit console.VoteAdderss(msg.sender, "msg.sender");
        emit console.VoteAdderss(tokenContract, "tokenContract");
        
        uint withdraw_amount = WithdrowRequestesErc20[msg.sender][tokenContract];
        emit console.VoteNumber(withdraw_amount, "withdraw amount");
        require(withdraw_amount > 0, "No withdrawal requests");

        IERC20 token = IERC20(tokenContract);
        uint allowance_amount = token.allowance(owner, address(this));
        require( allowance_amount >= withdraw_amount, "[erc20] Withdrawal is not allowed" );

        bool send = token.transferFrom(owner, msg.sender, withdraw_amount);
        require(send, "Failed to send");
        WithdrowRequestesErc20[msg.sender][tokenContract] = 0;

        return true;
    }
  
}
