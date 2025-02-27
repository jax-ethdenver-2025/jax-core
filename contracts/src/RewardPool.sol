// SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.28;

import {ERC20} from "solady/tokens/ERC20.sol";
import {Ownable} from "solady/auth/Ownable.sol";

contract RewardPool is Ownable {
    ERC20 public jaxToken;
    uint256 public bountyPerEpoch;
    mapping(address => uint256) public balances;
    mapping(address => uint256) public rewards;
    
    // Add storage for pool metadata
    string public contentHash;
    string public originatorNodeId;
    
    // Storage for historical peers
    mapping(string => bool) public peers;
    string[] public peerList;

    // Add initialized flag
    bool private initialized;

    event PeerAdded(string indexed nodeId);
    event PeerRemoved(string indexed nodeId);
    event Deposit(address indexed user, uint256 amount);
    event RewardDistributed(address indexed user, uint256 reward);

    constructor() {}

    function initialize(
        address _jaxToken,
        string memory _hash,
        string memory _originatorNodeId
    ) external {
        require(!initialized, "Already initialized");
        require(_jaxToken != address(0), "Invalid token address");
        require(bytes(_hash).length > 0, "Invalid hash");
        require(bytes(_originatorNodeId).length > 0, "Invalid originator");
        
        initialized = true;
        jaxToken = ERC20(_jaxToken);
        contentHash = _hash;
        originatorNodeId = _originatorNodeId;
    }

    // Add modifier for initialization check
    modifier whenInitialized() {
        require(initialized, "Not initialized");
        _;
    }

    function enterPool(string memory nodeId) external whenInitialized {
        require(!peers[nodeId], "Peer already in pool");
        require(bytes(nodeId).length > 0, "Invalid node ID");
        peers[nodeId] = true;
        peerList.push(nodeId);
        emit PeerAdded(nodeId);
    }

    // Add helper function to get all peers
    function getAllPeers() external view returns (string[] memory) {
        return peerList;
    }

    function deposit(uint256 amount) external whenInitialized {
        require(amount > 0, "Invalid amount");
        require(jaxToken.transferFrom(msg.sender, address(this), amount), "Deposit failed");
        balances[msg.sender] += amount;
        emit Deposit(msg.sender, amount);
    }

    function setBountyPerEpoch(uint256 _bounty) external onlyOwner whenInitialized {
        bountyPerEpoch = _bounty;
    }

    function distributeRewards() external whenInitialized {
        // Implement AVS logic here
        // For each user, calculate their reward based on AVS
        // Update their rewards mapping
        // Emit RewardDistributed event
    }
}