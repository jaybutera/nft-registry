### Build
```bash
./scripts/init.sh
cargo build --release
```

### Test
Run tests from the ```runtime``` package, not the root
```bash
cd ./runtime
cargo test
```

You can see the substrate runtime event logs with
```bash
cargo test -- --nocapture
```

### Runtime Design

The nftregistry substrate module allows new NFT registries to be ..registered.
Custom validation logic for minting new NFTs of a given registry is provided in
a WebAssembly contract that will be invoked by the runtime before minting.

#### User flow to create a new registry
1. User uploads and instantiates an instance of a WebAssembly contract that contains a validation method
(Details in [])
// 2. User encodes method selector to validation function with the Substrate SCALE
// encoding (details [])
2. User calls "new_registry" method of nftregistry module, providing the
address of the instantiated validation contract and encoded method selector

#### User flow to mint an NFT with an existing registry
1. User calls "mint" method of nftregistry module, providing registry uid to
register and encoded parameters for validation contract

Nomenclature:
Registry - A set of NFTs of a single class and a custom mint validation function
NFT - A specific token instance in a registry

The Centrifuge chain has a custom Nft Registry module that defines the
following dispatchable functions (meaning they can be called by users of the
blockchain):

- fn new_registry(validation_fn_addr: AccountId);
- fn mint(uid: Hash, parameters: Vec<u8>, value: BalanceOf, gas_limit: Gas);
- fn finish_mint(uid: &RegistryUid);

### New Registry
A registry is a mapping of a unique id to an address. The address should be a
smart contract that represents an NFT and has a validation function for minting
new tokens. Further measures can be taken to make guarantees that the addres
indeed points to a contract representing a type of NFT.

The new_registry function assigns a unique id to a validation contract address
and stores the key-value pair in a mapping, and finally emits an event.

### Mint
This function will invoke a call to the specified registry contract's
validation function, passing along any provided call data.

The function assumes the contract will in turn call `finish_mint` on a successful
validation.

### Finish Mint
Is intended to be called by the validation contract and **not directly by a
user**. Therefore it checks that the sender is the validator contract address of the
registry that was specified as an argument.

Once ensured, a unique id is generated for the new NFT and it is stored as a
mapping to a byte array that represents the NFT storage structure.

### Runtime vs Contract storage
In general there are two places to store data for a given NFT
1. Mappings in the runtime storage
2. The validation contract associated with a registry

Currently there are no unique data types for NFTs in the runtime - just a byte
array for each NFT. Data that you
want to control the behaviour of should go in the runtime. For instance, if you
know that an NFT will always have a single account owner, then the owner and
transfer logic should become part of the module storage and dispatch methods.
Otherwise, you could leave such details to be tracked in the validator contract
storage according to its own defined behaviours.

### Issues Run Into
I've run into some issues in the Substrate code. This is to be expected,
Substrate is a work in progress. But it just means the process is very involved
right now and you may end up needing to contribute fixes/enhancements. Also,
the generated Rust docs hosted on substrate.dev are for v1 of Substrate. The
best resource is to read the code straight from Github. The [Riot
channels](https://riot.im/app/#/room/#substrate-technical:matrix.org) are also
decent. There is one for Substrate and one for Ink!.

Currently, if you try to build the ink contract you will get an error that
`invoke_runtime` is not a function of `EnvAccessMut`. This is ready to be
fixed, see this [PR](https://github.com/paritytech/ink/pull/302). In the
meantime if you want to build the contract, you can clone the ink repository
and make the changes locally [].

Other related issues
https://github.com/paritytech/substrate/issues/4506
https://github.com/paritytech/ink/issues/301
https://github.com/paritytech/ink/issues/297
https://github.com/paritytech/ink-types-node-runtime/pull/7

### Failed attempts at custom validation
The method I converged on is to use the Wasm runtime interface function,
`dispatch_call`. Also called `invoke_runtime` through the ink env wrapper. The
contracts module exposes a set of these functions as a sort of system API for
ink contracts to control the more crtical parts of the runtime. For instance,
contracts can read and write runtime state through these methods. The complete
list is specified in the code at
[contracts/src/wasm/runtime.rs](https://github.com/paritytech/substrate/blob/40a16efefc070faf5a25442bc3ae1d0ea2478eee/frame/contracts/src/wasm/runtime.rs#L296)

There is also a document discussing the API design in v2
[here](https://hackmd.io/@robbepop/rkPdZHrrS#Current-State).

There were a few other ways to I tried to get communication between the
substrate runtime and Wasm contracts.

#### Events
Events are stored in a global list and therefore seemed like a good avenue to
extract data from a contract. Essentially the runtime calls the contract, the
contract emits an event specifying whether the validation was a success, and
the runtime then pulls the event off the stack and reads its contents.

In Substrate the Event enum for each module is rolled up into one aggregate
enum via a macro at compile time. When a contract emits an event it will be of
variant `Contract(..)` as specified
[here](https://substrate.dev/rustdocs/v1.0/srml_contract/enum.RawEvent.html).
But when it is stored in the global event log it is converted into a
system::Trait Event type, because that is the module which stores the event
log. Unfortunately the substrate primitive types do not support converting back
from a system trait and so the global variants cannot be matched on from a
different module (like nftregistry). A conversation with the devs on Riot
confirmed that this has never been considered and so I abandoned this option.

#### Return values from a dispatchable function
It seems that it would make sense to retrieve return data of a contract call
from the `contracts` `call` dispatch function
[srml_contract/lib.rs.html#335](https://substrate.dev/rustdocs/v1.0/src/srml_contract/lib.rs.html#335).
However, dispatch methods are apparently not intended to ever return useful
results in this way. I think the devs have future plans to make functions
asynchronous and maybe this would not play well together. Either way, if they
aren't willing to support the change then its a dead end unless we want to fork
Substrate.

### Tests
There are two complete tests in
[nftregistry/tests.rs](https://github.com/jaybutera/nft-registry/tree/master/runtime/src/nftregistry/tests.rs#L331).
Both upload a WebAssembly contract, create a registry, and mint
an NFT.

```mint_nft_from_basic_contract``` - Compiles a simple contract written
directly in WebAssembly to dispatch an encoded call to
`nftregistry::finish_mint`.
```mint_nft_from_ink_contract``` - Reads a compiled .wasm file of an ink
contract and attempts to invoke a method `validate` within it.

There is also a lot of boilerplate code in the same file. The Dummy structures are used for
behaviours needed in the contracts module to use u64 AccountIds instead of H256 like the
production runtime does. This is all just to make testing more simple and can
be ignored now that its defined.
