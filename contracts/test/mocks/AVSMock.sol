pragma solidity ^0.8.20;

import "../../src/interface/IIncredibleSquaringTaskManager.sol";

contract AVSMock is IIncredibleSquaringTaskManager {
    uint32 public latestTaskNum;

    mapping(uint32 => bytes32) public allTaskResponses;

    uint32 public immutable TASK_RESPONSE_WINDOW_BLOCK = 100;

    function createNewTask(bytes32 fileHash, uint32 quorumThresholdPercentage, bytes calldata quorumNumbers)
        external
        override
    {
        latestTaskNum++;
    }

    function taskNumber() external view override returns (uint32) {
        return latestTaskNum;
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
        return TASK_RESPONSE_WINDOW_BLOCK;
    }

    function getTaskResponse(uint32 taskIndex) external view override returns (TaskResponse memory) {
        return TaskResponse({referenceTaskIndex: taskIndex, providers: new address[](0), scores: new uint256[](0)});
    }
}
