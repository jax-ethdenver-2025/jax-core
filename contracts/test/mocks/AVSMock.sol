pragma solidity ^0.8.20;

import {IAVS} from "../../src/interface/IAVS.sol";

contract AVSMock is IAVS {
    function getWalletProviders() public pure override returns (address[] memory) {
        return new address[](0);
    }
}