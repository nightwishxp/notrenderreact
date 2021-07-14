
# Secret-SCRT - Privacy coin backed by SCRT

This is a privacy token implementation on the Secret Network. It is backed by
the native coin of the network (SCRT) and has a fixed 1-to-1 exchange ratio
with it.

Version 1.0.0 of this contract is deployed to mainnet at the address
`secret1k0jntykt7e4g3y88ltc60czgjuqdy4c9e8fzek`. The deployed binary can be
reproduced by checking out the commit tagged `v1.0.0` of this repository and
running the command `make compile-optimized-reproducible`.
See [Verifying build](#verifying-build) for full instructions of how to
verify the authenticity of the deployed binary.

Usage is pretty simple - you deposit SCRT into the contract, and you get SSCRT 
(or Secret-SCRT), which you can then use with the ERC-20-like functionality that
the contract provides including: sending/receiving/allowance and withdrawing
back to SCRT. 

In terms of privacy the deposit & withdrawals are public, as they are
transactions on-chain. The rest of the functionality is private (so no one can
see if you send SSCRT and to whom, and receiving SSCRT is also hidden). 

## Usage examples:

Usage examples here assume `v1.0.3` of the CLI is installed.
Users using `v1.0.2` of the CLI can instead send raw compute transactions
and queries based on the schema that the contract expects.

For full documentation see:
```
secretcli tx snip20 --help
secretcli q snip20 --help
```

To deposit: ***(This is public)***
```
secretcli tx snip20 deposit sscrt --amount 1000000uscrt --from <account>
```

To redeem: ***(This is public)***
```
secretcli tx snip20 redeem sscrt <amount-to-redeem> --from <account>
```

To send SSCRT: ***(Only you will be able to see the parameters you send here)***