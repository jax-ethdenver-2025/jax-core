// SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.28;

import {ERC20} from "solady/tokens/ERC20.sol";
import {Ownable} from "solady/auth/Ownable.sol";
import {Ed25519} from "./libraries/ED25519.sol";
import {IIncredibleSquaringTaskManager} from "./interface/IIncredibleSquaringTaskManager.sol";
struct Signature {
    bytes32 k;
    bytes32 r;
    bytes32 s;
    bytes m;
}

contract RewardPool is Ownable {
    IIncredibleSquaringTaskManager public avs;

    uint256 public bountyPerEpoch;
    mapping(address => uint256) public balances;
    mapping(address => uint256) public rewards;

    // Add storage for pool metadata
    string public contentHash;

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

    function initialize(address _avs, string memory _hash) external payable {
        // NOTE: idk if we need this
        // require(msg.value > 0, "Invalid amount");
        require(!initialized, "Already initialized");
        require(bytes(_hash).length > 0, "Invalid hash");

        initialized = true;
        contentHash = _hash;
        avs = IIncredibleSquaringTaskManager(_avs);

        balances[msg.sender] += msg.value;
        emit Deposit(msg.sender, msg.value);
    }

    // Add modifier for initialization check
    modifier whenInitialized() {
        require(initialized, "Not initialized");
        _;
    }

    function enterPool(string memory nodeId, Signature memory signature) external whenInitialized {
        require(!peers[nodeId], "Peer already in pool");
        require(bytes(nodeId).length > 0, "Invalid node ID");
        // TODO: add signature verification again
        // require(verify(signature.k, signature.r, signature.s, signature.m), "Invalid signature");
        peers[nodeId] = true;
        peerList.push(nodeId);
        emit PeerAdded(nodeId);
    }

    function getAllPeers() external view returns (string[] memory) {
        return peerList;
    }

    function getHash() external view returns (string memory) {
        return contentHash;
    }

    function deposit() external payable whenInitialized {
        require(msg.value > 0, "Invalid amount");
        balances[msg.sender] += msg.value;
        emit Deposit(msg.sender, msg.value);
    }

    function setBountyPerEpoch(uint256 _bounty) external onlyOwner whenInitialized {
        bountyPerEpoch = _bounty;
    }

    function verify(bytes32 k, bytes32 r, bytes32 s, bytes memory m) public pure returns (bool) {
        return Ed25519.verify(k, r, s, m);
    }

    function getTaskResponse(uint32 taskIndex) public view returns (IIncredibleSquaringTaskManager.TaskResponse memory) {
        return avs.getTaskResponse(taskIndex);
    }
}
