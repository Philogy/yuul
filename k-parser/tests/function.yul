function abi_decode_address() -> value
{
    value := calldataload(4)
    if iszero(eq(value, and(value, sub(shl(160, 1), 1)))) { revert(0, 0) }
}
