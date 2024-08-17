case 0x70a08231 {
    if callvalue() { revert(0, 0) }
    if slt(add(calldatasize(), not(3)), 32) { revert(0, 0) }
    mstore(0, and(abi_decode_address(), sub(shl(160, 1), 1)))
    mstore(32, 0)
    mstore(_1, sload(keccak256(0, 64)))
    return(_1, 32)
}
