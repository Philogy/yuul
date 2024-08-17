/// @use-src 0:"src/ERC20.sol"
object "ERC20_39" {
    code {
        {
            /// @src 0:106:385  "contract ERC20 {..."
            let _1 := memoryguard(0x80)
            mstore(64, _1)
            if callvalue() { revert(0, 0) }
            mstore(/** @src 0:202:211  "balanceOf" */ 0x00, /** @src 0:212:222  "msg.sender" */ caller())
            /// @src 0:106:385  "contract ERC20 {..."
            mstore(0x20, /** @src 0:202:211  "balanceOf" */ 0x00)
            /// @src 0:106:385  "contract ERC20 {..."
            let dataSlot := keccak256(/** @src 0:202:211  "balanceOf" */ 0x00, /** @src 0:106:385  "contract ERC20 {..." */ 64)
            let _2 := sload(/** @src 0:202:234  "balanceOf[msg.sender] += 1000e18" */ dataSlot)
            /// @src 0:106:385  "contract ERC20 {..."
            let sum := add(_2, /** @src 0:227:234  "1000e18" */ 0x3635c9adc5dea00000)
            /// @src 0:106:385  "contract ERC20 {..."
            if gt(_2, sum)
            {
                mstore(/** @src 0:202:211  "balanceOf" */ 0x00, /** @src 0:106:385  "contract ERC20 {..." */ shl(224, 0x4e487b71))
                mstore(4, 0x11)
                revert(/** @src 0:202:211  "balanceOf" */ 0x00, /** @src 0:106:385  "contract ERC20 {..." */ 0x24)
            }
            sstore(dataSlot, sum)
            let _3 := datasize("ERC20_39_deployed")
            codecopy(_1, dataoffset("ERC20_39_deployed"), _3)
            return(_1, _3)
        }
    }
    /// @use-src 0:"src/ERC20.sol"
    object "ERC20_39_deployed" {
        code {
            {
                /// @src 0:106:385  "contract ERC20 {..."
                let _1 := memoryguard(0x80)
                mstore(64, _1)
                if iszero(lt(calldatasize(), 4))
                {
                    switch shr(224, calldataload(0))
                    case 0x70a08231 {
                        if callvalue() { revert(0, 0) }
                        if slt(add(calldatasize(), not(3)), 32) { revert(0, 0) }
                        mstore(0, and(abi_decode_address(), sub(shl(160, 1), 1)))
                        mstore(32, 0)
                        mstore(_1, sload(keccak256(0, 64)))
                        return(_1, 32)
                    }
                    case 0xa9059cbb {
                        if callvalue() { revert(0, 0) }
                        if slt(add(calldatasize(), not(3)), 64) { revert(0, 0) }
                        let value0 := abi_decode_address()
                        let value := calldataload(36)
                        mstore(0, /** @src 0:322:332  "msg.sender" */ caller())
                        /// @src 0:106:385  "contract ERC20 {..."
                        mstore(32, 0)
                        let dataSlot := keccak256(0, 64)
                        let _2 := sload(/** @src 0:312:343  "balanceOf[msg.sender] -= amount" */ dataSlot)
                        /// @src 0:106:385  "contract ERC20 {..."
                        let diff := sub(_2, value)
                        if gt(diff, _2)
                        {
                            mstore(0, shl(224, 0x4e487b71))
                            mstore(4, 0x11)
                            revert(0, 36)
                        }
                        sstore(dataSlot, diff)
                        mstore(0, and(value0, sub(shl(160, 1), 1)))
                        mstore(32, 0)
                        let dataSlot_1 := keccak256(0, 64)
                        let _3 := sload(/** @src 0:353:376  "balanceOf[to] += amount" */ dataSlot_1)
                        /// @src 0:106:385  "contract ERC20 {..."
                        let sum := add(_3, value)
                        if gt(_3, sum)
                        {
                            mstore(0, shl(224, 0x4e487b71))
                            mstore(4, 0x11)
                            revert(0, 36)
                        }
                        sstore(dataSlot_1, sum)
                        return(0, 0)
                    }
                }
                revert(0, 0)
            }
            function abi_decode_address() -> value
            {
                value := calldataload(4)
                if iszero(eq(value, and(value, sub(shl(160, 1), 1)))) { revert(0, 0) }
            }
        }
        data ".metadata" hex"a26469706673582212200ef641ec034afa9d6de4f0ab7e06def33c8d2f9dcb0e455b29e6e4971f31660b64736f6c634300081a0033"
    }
}


