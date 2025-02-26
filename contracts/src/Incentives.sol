// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import {ERC20} from "solady/tokens/ERC20.sol";

contract Incentives {
    ERC20 public jaxtoken;

    event Deposit(address indexed sender, uint256 amount);
    event Withdrawal(address indexed recipient, uint256 amount);

    // Set the token address (JaxToken) at deployment
    constructor(address tokenAddress) {
        jaxtoken = ERC20(tokenAddress);
    }

    // Allows users to deposit tokens into the faucet.
    // Make sure to approve this contract to spend your tokens before calling.
    function deposit(uint256 amount) external {
        require(jaxtoken.transferFrom(msg.sender, address(this), amount), "Deposit failed");
        emit Deposit(msg.sender, amount);
    }

    // Allows users to withdraw tokens from the faucet.
    function withdraw(uint256 amount) external {
        require(jaxtoken.balanceOf(address(this)) >= amount, "Not enough tokens in faucet");
        require(jaxtoken.transfer(msg.sender, amount), "Withdrawal failed");
        emit Withdrawal(msg.sender, amount);
    }
}
