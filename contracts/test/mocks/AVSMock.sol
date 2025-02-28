pragma solidity ^0.8.20;

import "../src/interface/IIncredibleSquaringTaskManager.sol";

contract AVSMock is IIncredibleSquaringTaskManager {
    function createNewTask(bytes32 fileHash, uint32 quorumThresholdPercentage, bytes calldata quorumNumbers) external override {
        // Mock implementation
    }

    function taskNumber() external view override returns (uint32) {
        return 0; // Mock implementation
    }

    function raiseAndResolveChallenge(
        Task calldata task,
        TaskResponse calldata taskResponse,
        TaskResponseMetadata calldata taskResponseMetadata,
        BN254.G1Point[] memory pubkeysOfNonSigningOperators
    ) external override {
        // Mock implementation
    }

    function getTaskResponseWindowBlock() external view override returns (uint32) {
        return 0; // Mock implementation
    }
}
