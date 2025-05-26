// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "contract/console.sol";
import "contract/IERC20.sol";

// deposit l1 => l2
// Withdraw l2 => l1
contract Bridge {
    address owner;
    // the commission for the creation of the bridge
    uint constant creation_commission_bridge = 1 ether;
    uint constant max_amount = 18_446_744_073_709_551_615;

    struct BridgeTokenInfo {
        bool turn;
        string name;
        string symbol;
        uint8 base_decimals;
        uint8 decimals;
    }

    event EventCreateBridge(BridgeTokenInfo);
    event EventDeposit(address from, address to, uint64 value);
    event EventDepositRC20(
        address token_address,
        address from, 
        address to, 
        uint value
    );

    mapping (address => BridgeTokenInfo) BridgeTokens;
    mapping (address => uint) WithdrowRequest;
    mapping (address =>  mapping (address => uint)) WithdrowRequestErc20;

    constructor() {
        owner = msg.sender;
    }

    receive() external payable {}

    function uint2str(uint _i) internal pure returns (string memory _uintAsString) {
        if (_i == 0) {
            return "0";
        }
        uint j = _i;
        uint len;
        while (j != 0) {
            len++;
            j /= 10;
        }
        bytes memory bstr = new bytes(len);
        uint k = len;
        while (_i != 0) {
            k = k-1;
            uint8 temp = (48 + uint8(_i - _i / 10 * 10));
            bytes1 b1 = bytes1(temp);
            bstr[k] = b1;
            _i /= 10;
        }
        return string(bstr);
    }

    function convert_amount(uint amount, uint8 from, uint8 to) public pure returns (uint) {
        if (from == to){
            return amount;
        }else if (from > to ){
            uint r = 10**(from - to);
            uint new_amount = amount/r;
            
            require(amount == new_amount*r, string.concat("Couldn't round up to the precision of ", uint2str(to)));
            
            return new_amount;
        }else{
            return amount * (10**(to-from));
        }
    }

    // (ETH) l1 => l2
    // decimal: 8 
    // max: uint64
    function deposit(address receiver) payable public returns (bool) {
        uint256 amount_deposit = msg.value;
        uint64 amount64 = uint64(convert_amount(amount_deposit, 18, 8));
        emit EventDeposit(msg.sender, receiver, amount64);
        return true;
    }

    // (ETH) L2 => L1
    // Owner: reserves coins for the client
    function request_withdraw(address to, uint64 amount) payable public returns (bool) {
        require(msg.sender == owner, "This request can only be completed by the owner");
        WithdrowRequest[to] = WithdrowRequest[to] + convert_amount(amount, 8, 18);
        return true;
    }

    function status_withdraw() public view returns (uint) {
        return WithdrowRequest[msg.sender];
    }

    // (ETH) Withdraw => Client
    // The client withdraws the coins to his account
    function withdraw() payable public returns (bool) {
        require(WithdrowRequest[msg.sender] > 0, "There are no funds for withdrawal");
        uint amount = WithdrowRequest[msg.sender];

        require(address(this).balance > amount, "Insufficient funds in the wallet");
        payable(msg.sender).transfer(amount);
        WithdrowRequest[msg.sender] = 0;

        return true;
    }

    // # # # ERC20

    function convert_amount_to_l2(address tokenContract, uint amount) public view returns (uint) {
        BridgeTokenInfo memory settings = BridgeTokens[tokenContract];
        require(settings.turn, "The bridge has not been created yet");

        amount = convert_amount(amount, settings.base_decimals, settings.decimals);
        require(max_amount >= amount, "The amount exceeds the allowed maximum");

        return amount;
    }

    function convert_amount_to_l1(address tokenContract, uint amount) public view returns (uint) {
        BridgeTokenInfo memory settings = BridgeTokens[tokenContract];
        require(settings.turn, "The bridge has not been created yet");

        return convert_amount(amount, settings.decimals, settings.base_decimals);
    }
    
    function exist_bridge_erc20(address tokenContract) public view returns (bool) {
        return BridgeTokens[tokenContract].turn;
    }

    function status_bridge_erc20(address tokenContract)public view returns (BridgeTokenInfo memory) {
        return BridgeTokens[tokenContract];
    }

    function create_bridge_erc20(address tokenContract) payable public returns (bool) {
        require(!BridgeTokens[tokenContract].turn, "The token has already been added");

        uint256 amount_deposit = msg.value;
        require(amount_deposit == creation_commission_bridge, "A 1 ETH commission is required to create a bridge.");

        IERC20 token = IERC20(tokenContract);
        string memory name = token.name();
        string memory symbol = token.symbol();
        uint8 base_decimals = token.decimals();

        uint8 decimals;
        if( base_decimals > 8 ){
            decimals = 8;
        }else{
            decimals = base_decimals;
        }

        BridgeTokenInfo memory bridge = BridgeTokenInfo ({
            turn: true,
            name: name,
            symbol: symbol,
            base_decimals: base_decimals,
            decimals: decimals
        });
        BridgeTokens[tokenContract] = bridge;

        emit EventCreateBridge(bridge);

        return true;
    }

    // (ERC20) l1 => l2
    function deposit_erc20(address tokenContract, address receiver, uint amount_deposit ) public returns (bool) {
        uint64 l2_amount = uint64(convert_amount_to_l2(tokenContract, amount_deposit));

        IERC20 token = IERC20(tokenContract);

        uint allow = token.allowance(msg.sender, address(this));
        require(allow >= amount_deposit ,"the transfer must be approved");

        bool send = token.transferFrom(msg.sender, address(this), amount_deposit);
        require(send, "Failed to send");

        emit EventDepositRC20(tokenContract, msg.sender, receiver, l2_amount);
        return true;
    }

    // (ERC20) L2 => L1
    // Owner: reserves coins for the client
    function request_withdraw_erc20(address tokenContract, address to, uint64 amount) payable public returns (bool){
        require(msg.sender == owner, "This request can only be completed by the owner");

        uint new_amount = convert_amount_to_l1(tokenContract, uint(amount));

        IERC20 token = IERC20(tokenContract);

        uint balance = token.balanceOf(address(this));
        require(balance > new_amount, "Insufficient funds to transfer funds from the wallet to the user");

        WithdrowRequestErc20[to][tokenContract] = WithdrowRequestErc20[to][tokenContract] + new_amount;
        
        return true;
    }

    function status_withdraw_erc20(address tokenContract) public view returns (uint){
        return WithdrowRequestErc20[msg.sender][tokenContract];
    }

    // (ERC20) Withdraw => Client
    // The client withdraws the coins to his account
    function withdraw_erc20(address tokenContract) payable public returns (bool){
        require(BridgeTokens[tokenContract].turn, "The bridge has not been created yet");
        
        uint withdraw_amount = WithdrowRequestErc20[msg.sender][tokenContract];
        require(withdraw_amount > 0, "No withdrawal requests");

        IERC20 token = IERC20(tokenContract);
        bool send = token.transfer(msg.sender, withdraw_amount);
        require(send, "Failed to send");

        WithdrowRequestErc20[msg.sender][tokenContract] = 0;

        return true;
    }
}
