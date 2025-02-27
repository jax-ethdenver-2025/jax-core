// SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.28;

import {ERC20} from "solady/tokens/ERC20.sol";
import {Ownable} from "solady/auth/Ownable.sol";

contract RewardPool is Ownable {
    ERC20 public jaxToken;
    uint256 public bountyPerEpoch;
    mapping(address => uint256) public balances;
    mapping(address => uint256) public rewards;

    event PeerAdded(string indexed nodeId);
    event PeerRemoved(string indexed nodeId);
    event Deposit(address indexed user, uint256 amount);
    event RewardDistributed(address indexed user, uint256 reward);

    constructor() {}

    function initialize(address _jaxToken) external {
        jaxToken = ERC20(_jaxToken);
    }

    function enterPool(string memory nodeId) external {
        emit PeerAdded(nodeId);
    }

    function deposit(uint256 amount) external {
        require(jaxToken.transferFrom(msg.sender, address(this), amount), "Deposit failed");
        balances[msg.sender] += amount;
        emit Deposit(msg.sender, amount);
    }

    function setBountyPerEpoch(uint256 _bounty) external onlyOwner {
        bountyPerEpoch = _bounty;
    }

    function distributeRewards() external {
        // Implement AVS logic here
        // For each user, calculate their reward based on AVS
        // Update their rewards mapping
        // Emit RewardDistributed event
    }
}