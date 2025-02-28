pragma solidity ^0.8.20;

import {IAVS} from "../../src/interface/IAVS.sol";

contract AVSMock is IAVS {
    function verify(bytes32 data) public pure override returns (bool) {
        return true;
    }

    function getWalletProviders() public pure override returns (address[] memory) {
        return new address[](0);
    }
}