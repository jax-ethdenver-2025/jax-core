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

    // Add storage for pool metadata
    bytes32 public contentHash;

    // Storage for historical peers
    mapping(string => bool) public peers;
    string[] public peerList;

    // whether or not the pool has been initialized
    bool private initialized;

    event PeerAdded(string indexed nodeId);
    event PeerRemoved(string indexed nodeId);
    // NOTE: we should probably not pass hash and do better
    //  indexing off chain
    event Deposit(uint256 amount, bytes32 hash);
    event RewardDistributed(address indexed user, uint256 reward);

    /* initializer / constructor */

    constructor() {}

    function initialize(address _avs, bytes32 _hash) external payable {
        // NOTE: idk if we need this
        // require(msg.value > 0, "Invalid amount");
        require(!initialized, "Already initialized");

        initialized = true;
        contentHash = _hash;
        avs = IIncredibleSquaringTaskManager(_avs);

        emit Deposit(msg.value, _hash);
    }

    // Add modifier for initialization check
    modifier whenInitialized() {
        require(initialized, "Not initialized");
        _;
    }

    /* state changing functions */

    // TODO: fix this / diagnose why we revert against rust bindings
    function enterPool(string memory nodeId, bytes32 k, bytes32 r, bytes32 s, address beneficiary) external whenInitialized {
        bytes memory m = abi.encodePacked(beneficiary);

        require(!peers[nodeId], "Peer already in pool");
        require(bytes(nodeId).length > 0, "Invalid node ID");
        require(verify(k, r, s, m), "Invalid signature");
        peers[nodeId] = true;
        peerList.push(nodeId);
        emit PeerAdded(nodeId);
    }

    function deposit() external payable whenInitialized {
        require(msg.value > 0, "Invalid amount");
        emit Deposit(msg.value, contentHash);
    }

    // TODO: distribute rewards and interface with the avs

    /* view functions */

    function getPeers() external view returns (string[] memory) {
        return peerList;
    }

    function getHash() external view returns (bytes32) {
        return contentHash;
    }

    function getBalance() external view returns (uint256) {
        return address(this).balance;
    }

    /* helper functions */

    function verify(bytes32 k, bytes32 r, bytes32 s, bytes memory m) public pure returns (bool) {
        return Ed25519.verify(k, r, s, m);
    }

    function getTaskResponse(uint32 taskIndex)
        public
        view
        returns (IIncredibleSquaringTaskManager.TaskResponse memory)
    {
        return avs.getTaskResponse(taskIndex);
    }
}
