/// A runtime module template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references


/// For more guidance on Substrate modules, see the example module
/// https://github.com/paritytech/substrate/blob/master/srml/example/src/lib.rs

use frame_support::{
    decl_module,
    ensure,
    decl_storage,
    decl_event,
    StorageValue,
    StorageMap,
    dispatch::Result};
use sp_runtime::traits::StaticLookup;
use system::{ensure_signed, RawOrigin};

use sp_std::prelude::*;

struct NftContract;

// A unique id for a NFT type (not an NFT instance)
//type RegistryUid<T: Trait> = T::Hash;
type RegistryUid = u64;
type NftUid = u64;

pub trait Trait: balances::Trait + contracts::Trait  {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event! {
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        //<T as system::Trait>::Hash,
    {
        NewRegistry(AccountId, RegistryUid),
        MintNft(RegistryUid, NftUid),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as NftRegistry {
        ValidationFn get(validator_of): map RegistryUid => Option<T::AccountId>;
        Nonce: u64;
        RegistryNonce: map RegistryUid => u64;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        fn new_registry(origin, validation_fn_addr: T::AccountId) -> Result {
            let sender = ensure_signed(origin)?;

            // Generate a uid and check that it's unique
            let nonce = <Nonce>::get();
            let uid = nonce;
            //let uid = (nonce).using_encoded(<T as system::Trait>::Hashing::hash);
            //ensure!(!<ValidationFn<T>>::exists(uid), "This new id for a registry already exists!");

            // Check for overflow on index
            let nplus1 = <Nonce>::get().checked_add(1)
                .ok_or("Nonce overflow when adding a new registry")?;

            // Write state
            <ValidationFn<T>>::insert(&uid, validation_fn_addr);
            <Nonce>::put( nplus1 );

            // Events
            Self::deposit_event(RawEvent::NewRegistry(sender, uid));

            Ok(())
        }

        fn mint(origin,
                uid: RegistryUid,
                parameters: Vec<u8>,            // To be passed into the smart contract
                value: contracts::BalanceOf<T>,  // If currency needs to be passed to contract
                gas_limit: contracts::Gas) -> Result
        {
            // TODO: Needs to ensure signed before anything else
            let sender = ensure_signed(origin)?;

            ensure!(<ValidationFn<T>>::exists(uid), "No registry with this uid exists");

            // Run merkle validation

            // Run custom validation
            let validation_fn = Self::validator_of(uid)
                .ok_or("This should not happen bcs ensure above^")?;

            // Wasm contract should emit an event for success or failure
            <contracts::Module<T>>::call(
                T::Origin::from(RawOrigin::<T::AccountId>::Signed(sender)),
                T::Lookup::unlookup(validation_fn.clone()),
                value,
                gas_limit,
                parameters)
        }

        fn finish_mint(origin, uid: RegistryUid) -> Result {
            let sender = ensure_signed(origin)?;

            // Ensure the caller is the validation contract for the corresponding NFT class
            ensure!(Self::validator_of(&uid)
                        .map_or(false, |validator_addr| validator_addr == sender),
                        "Sender must be validator contract for this Nft registry");

            // Assign Nft uid
            let nft_uid = <RegistryNonce>::get(&uid);

            let nplus1 = nft_uid.checked_add(1)
                .ok_or("Overflow when incrementing registry nonce")?;

            // Increment nonce
            <RegistryNonce>::insert(uid, nplus1);

            // Just emit an event
            Self::deposit_event(RawEvent::MintNft(uid, nft_uid));

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::{
        impl_outer_origin, impl_outer_event, impl_outer_dispatch, assert_ok,
        parameter_types, weights::Weight, traits::Currency,
    };
    use std::cell::RefCell;
    use sp_core::{Blake2Hasher, sr25519};
    use sp_runtime::{
        BuildStorage, traits::{BlakeTwo256, IdentityLookup, Hash},
        testing::{Digest, DigestItem, Header, H256},
        Perbill,
    };
    use sp_version::RuntimeVersion;
    use contracts::{TrieIdGenerator, TrieId, ComputeDispatchFee, ContractAddressFor, AccountCounter};
    use ink_core::env2::call::{Selector, CallData};

    mod nftregistry {
        // Re-export contents of the root. This basically
        // needs to give a name for the current crate.
        // This hack is required for `impl_outer_event!`.
        pub use super::super::*;
        use frame_support::impl_outer_event;
    }

    #[derive(Eq, Clone, PartialEq)]
    pub struct NftRegistryTest;

    impl_outer_origin! {
        pub enum Origin for NftRegistryTest {}
    }

    impl_outer_dispatch! {
        pub enum Call for NftRegistryTest where origin: Origin {
            balances::Balances,
            contracts::Contract,
            nftregistry::NftRegistry,
        }
    }

    impl_outer_event! {
        pub enum MetaEvent for NftRegistryTest {
            balances<T>, contracts<T>, nftregistry<T>,
        }
    }

    type BlockNumber = u64;

    parameter_types! {
        pub const BlockHashCount: BlockNumber = 250;
        pub const MaximumBlockWeight: Weight = 1_000_000;
        pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
        pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
    }

    impl system::Trait for NftRegistryTest {
        type Call = ();
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Index = u64;
        type BlockNumber = BlockNumber;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Header = Header;
        type Event = MetaEvent;
        type Origin = Origin;
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
    }

    parameter_types! {
        pub const ExistentialDeposit: u64 = 500;
        pub const TransferFee: u64 = 0;
        pub const CreationFee: u64 = 0;
    }

    impl balances::Trait for NftRegistryTest {
        type Balance = u64;
        type OnFreeBalanceZero = contracts::Module<NftRegistryTest>;
        type OnNewAccount = ();
        type Event = MetaEvent;
        type DustRemoval = ();
        type TransferPayment = ();
        type ExistentialDeposit = ExistentialDeposit;
        type TransferFee = TransferFee;
        type CreationFee = CreationFee;
    }

    parameter_types! {
        pub const MinimumPeriod: u64 = 1;
    }

    impl timestamp::Trait for NftRegistryTest {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = MinimumPeriod;
    }
    parameter_types! {
        pub const SignedClaimHandicap: u32 = 2;
        pub const TombstoneDeposit: u64 = 16;
        pub const StorageSizeOffset: u32 = 8;
        pub const RentByteFee: u64 = 4;
        pub const RentDepositOffset: u64 = 10_000;
        pub const SurchargeReward: u64 = 150;
        pub const TransactionBaseFee: u64 = 2;
        pub const TransactionByteFee: u64 = 6;
        pub const ContractFee: u64 = 21;
        pub const CallBaseFee: u64 = 135;
        pub const InstantiateBaseFee: u64 = 175;
        pub const MaxDepth: u32 = 100;
        pub const MaxValueSize: u32 = 16_384;
        pub const BlockGasLimit: u64 = 100_000;
    }

    pub struct ExtBuilder {
        existential_deposit: u64,
        gas_price: u64,
        block_gas_limit: u64,
        transfer_fee: u64,
        instantiation_fee: u64,
    }
    impl Default for ExtBuilder {
        fn default() -> Self {
            Self {
                existential_deposit: 0,
                gas_price: 2,
                block_gas_limit: 100_000_000,
                transfer_fee: 0,
                instantiation_fee: 0,
            }
        }
    }
    thread_local! {
        static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(0);
        static TRANSFER_FEE: RefCell<u64> = RefCell::new(0);
        static INSTANTIATION_FEE: RefCell<u64> = RefCell::new(0);
        static BLOCK_GAS_LIMIT: RefCell<u64> = RefCell::new(0);
    }
    impl ExtBuilder {
        pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
            self.existential_deposit = existential_deposit;
            self
        }
        pub fn gas_price(mut self, gas_price: u64) -> Self {
            self.gas_price = gas_price;
            self
        }
        pub fn block_gas_limit(mut self, block_gas_limit: u64) -> Self {
            self.block_gas_limit = block_gas_limit;
            self
        }
        pub fn transfer_fee(mut self, transfer_fee: u64) -> Self {
            self.transfer_fee = transfer_fee;
            self
        }
        pub fn instantiation_fee(mut self, instantiation_fee: u64) -> Self {
            self.instantiation_fee = instantiation_fee;
            self
        }
        pub fn set_associated_consts(&self) {
            EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
            TRANSFER_FEE.with(|v| *v.borrow_mut() = self.transfer_fee);
            INSTANTIATION_FEE.with(|v| *v.borrow_mut() = self.instantiation_fee);
            BLOCK_GAS_LIMIT.with(|v| *v.borrow_mut() = self.block_gas_limit);
        }
        pub fn build(self) -> sp_io::TestExternalities {
            self.set_associated_consts();
            let mut t = system::GenesisConfig::default().build_storage::<NftRegistryTest>().unwrap();
            balances::GenesisConfig::<NftRegistryTest> {
                balances: vec![],
                vesting: vec![],
            }.assimilate_storage(&mut t).unwrap();
            contracts::GenesisConfig::<NftRegistryTest> {
                current_schedule: contracts::Schedule {
                    enable_println: true,
                    ..Default::default()
                },
                gas_price: self.gas_price.into(),
            }.assimilate_storage(&mut t).unwrap();
            sp_io::TestExternalities::new(t)
        }
    }

    type Timestamp = timestamp::Module<NftRegistryTest>;
    type NftReg = super::Module<NftRegistryTest>;
    type Balances = balances::Module<NftRegistryTest>;
    type Contract = contracts::Module<NftRegistryTest>;
    type NftRegistry = super::Module<NftRegistryTest>;

    impl contracts::Trait for NftRegistryTest {
        type Currency = Balances;
        type Time = Timestamp;
        type Randomness = randomness_collective_flip::Module<NftRegistryTest>;
        type Call = Call;
        type Event = MetaEvent;
        type DetermineContractAddress = DummyContractAddressFor;
        type ComputeDispatchFee = DummyComputeDispatchFee;
        type TrieIdGenerator = DummyTrieIdGenerator;
        type GasPayment = ();
        type RentPayment = ();
        type SignedClaimHandicap = SignedClaimHandicap;
        type TombstoneDeposit = TombstoneDeposit;
        type StorageSizeOffset = StorageSizeOffset;
        type RentByteFee = RentByteFee;
        type RentDepositOffset = RentDepositOffset;
        type SurchargeReward = SurchargeReward;
        type TransferFee = TransferFee;
        type CreationFee = CreationFee;
        type TransactionBaseFee = TransactionBaseFee;
        type TransactionByteFee = TransactionByteFee;
        type ContractFee = ContractFee;
        type CallBaseFee = CallBaseFee;
        type InstantiateBaseFee = InstantiateBaseFee;
        type MaxDepth = MaxDepth;
        type MaxValueSize = MaxValueSize;
        type BlockGasLimit = BlockGasLimit;
    }

    impl super::Trait for NftRegistryTest {
        type Event = MetaEvent;
    }


    pub struct DummyContractAddressFor;
    impl ContractAddressFor<H256, u64> for DummyContractAddressFor {
        fn contract_address_for(_code_hash: &H256, _data: &[u8], origin: &u64) -> u64 {
            *origin + 1
        }
    }

    pub struct DummyTrieIdGenerator;
    impl TrieIdGenerator<u64> for DummyTrieIdGenerator {
        fn trie_id(account_id: &u64) -> TrieId {
            use sp_core::storage::well_known_keys;

            let new_seed = contracts::AccountCounter::mutate(|v| {
                *v = v.wrapping_add(1);
                *v
            });

            let mut res = vec![];
            res.extend_from_slice(well_known_keys::CHILD_STORAGE_KEY_PREFIX);
            res.extend_from_slice(b"default:");
            res.extend_from_slice(&new_seed.to_le_bytes());
            res.extend_from_slice(&account_id.to_le_bytes());
            res
        }
    }

    pub struct DummyComputeDispatchFee;
    impl ComputeDispatchFee<Call, u64> for DummyComputeDispatchFee {
        fn compute_dispatch_fee(call: &Call) -> u64 {
            69
        }
    }


    const ALICE: u64 = 1;
    const BOB: u64 = 2;
    const CHARLIE: u64 = 3;
    const DJANGO: u64 = 4;
    const CODE_SET_RENT: &str = r#"
(module
	(import "env" "ext_dispatch_call" (func $ext_dispatch_call (param i32 i32)))
	(import "env" "ext_set_storage" (func $ext_set_storage (param i32 i32 i32 i32)))
	(import "env" "ext_set_rent_allowance" (func $ext_set_rent_allowance (param i32 i32)))
	(import "env" "ext_scratch_size" (func $ext_scratch_size (result i32)))
	(import "env" "ext_scratch_read" (func $ext_scratch_read (param i32 i32 i32)))
	(import "env" "memory" (memory 1 1))
	;; insert a value of 4 bytes into storage
	(func $call_0
		(call $ext_set_storage
			(i32.const 1)
			(i32.const 1)
			(i32.const 0)
			(i32.const 4)
		)
	)
	;; remove the value inserted by call_1
	(func $call_1
		(call $ext_set_storage
			(i32.const 1)
			(i32.const 0)
			(i32.const 0)
			(i32.const 0)
		)
	)
	;; transfer 50 to ALICE
	(func $call_2
		(call $ext_dispatch_call
			(i32.const 68)
			(i32.const 11)
		)
	)
	;; do nothing
	(func $call_else)
	(func $assert (param i32)
		(block $ok
			(br_if $ok
				(get_local 0)
			)
			(unreachable)
		)
	)
	;; Dispatch the call according to input size
	(func (export "call")
		(local $input_size i32)
		(set_local $input_size
			(call $ext_scratch_size)
		)
		(block $IF_ELSE
			(block $IF_2
				(block $IF_1
					(block $IF_0
						(br_table $IF_0 $IF_1 $IF_2 $IF_ELSE
							(get_local $input_size)
						)
						(unreachable)
					)
					(call $call_0)
					return
				)
				(call $call_1)
				return
			)
			(call $call_2)
			return
		)
		(call $call_else)
	)
	;; Set into storage a 4 bytes value
	;; Set call set_rent_allowance with input
	(func (export "deploy")
		(local $input_size i32)
		(set_local $input_size
			(call $ext_scratch_size)
		)
		(call $ext_set_storage
			(i32.const 0)
			(i32.const 1)
			(i32.const 0)
			(i32.const 4)
		)
		(call $ext_scratch_read
			(i32.const 0)
			(i32.const 0)
			(get_local $input_size)
		)
		(call $ext_set_rent_allowance
			(i32.const 0)
			(get_local $input_size)
		)
	)
	;; Encoding of 10 in balance
	(data (i32.const 0) "\28")
	;; Encoding of call transfer 50 to CHARLIE
	(data (i32.const 68) "\00\00\03\00\00\00\00\00\00\00\C8")
)
"#;

    const CODE_DISPATCH_CALL: &str = r#"
(module
	(import "env" "ext_dispatch_call" (func $ext_dispatch_call (param i32 i32)))
	(import "env" "memory" (memory 1 1))
	(func (export "call")
		(call $ext_dispatch_call
			(i32.const 8) ;; Pointer to the start of encoded call buffer
			(i32.const 11) ;; Length of the buffer
		)
	)
	(func (export "deploy"))
	(data (i32.const 8) "\00\00\03\00\00\00\00\00\00\00\C8")
)
"#;

    fn compile_module<T>(wabt_module: &str)
        -> std::result::Result<(Vec<u8>, <T::Hashing as Hash>::Output), wabt::Error>
        where T: system::Trait
    {
        let wasm = wabt::wat2wasm(wabt_module)?;
        let code_hash = T::Hashing::hash(&wasm);
        Ok((wasm, code_hash))
    }

    fn get_wasm_bytecode() -> std::result::Result<Vec<u8>, &'static str> {
        use std::{io, io::prelude::*, fs::File};
        use std::path::Path;

        /*
        // Get the wasm contract byte code from a file
        let mut f = File::open(Path::new("./testcontract.wasm"))
            .map_err(|_| "Failed to open contract file")?;
        let mut bytecode = Vec::<u8>::new();
        f.read_to_end(&mut bytecode)
            .map(|_| bytecode)
            .map_err(|_| "Didn't read to end of file")
        */

        let (bytecode, codehash) = compile_module::<NftRegistryTest>(CODE_SET_RENT).unwrap();
        Ok(bytecode)
    }

    fn init_contract(origin: Origin) -> Result {
        let bytecode = get_wasm_bytecode()?;
        //let codehash = <NftRegistryTest as system::Trait>::Hashing::hash(&bytecode);
        //let codehash = (bytecode).using_encoded(<T as system::Trait>::Hashing::hash);
        //let (bytecode, codehash) = compile_module::<NftRegistryTest>(CODE_SET_RENT).unwrap();

        // Store code on chain
        <contracts::Module<NftRegistryTest>>::put_code(
            origin.clone(),
            100_000,
            bytecode
            )?;

        // Get codehash from event log
        let codehash_event = <system::Module<NftRegistryTest>>::events().pop()
            .ok_or("An event should be in the log but its not")?;
        let codehash = match codehash_event.event {
            MetaEvent::contracts(contracts::RawEvent::CodeStored(hash)) => Some(hash),
            _ => None,
        }.ok_or("Latest event is not a CodeStored event")?;

        println!("codehash: {:?}", codehash.clone());

        // Instantiate contract
        <contracts::Module<NftRegistryTest>>::instantiate(origin, 1_000, 100_000, codehash, codec::Encode::encode(&ALICE))
        //<contracts::Module<NftRegistryTest>>::instantiate(origin, 1_000, 100_000, codehash, vec![])
    }

    #[test]
    fn create_nft_registry() {
        ExtBuilder::default().build().execute_with(|| {
            Balances::deposit_creating(&ALICE, 100_000_000);
            let origin = Origin::signed(ALICE);
            let registry_id = 0;

            println!("Free bal: {}", <balances::Module<NftRegistryTest>>::free_balance(&ALICE));

            assert_ok!(
                init_contract( origin.clone() )
            );

            // Call validation contract method
            let mut call = CallData::new( Selector::from_str("validate") );
            call.push_arg(&codec::Encode::encode(&ALICE));
            call.push_arg(&registry_id);

            let bytecode = get_wasm_bytecode().unwrap();
            let codehash = <NftRegistryTest as system::Trait>::Hashing::hash(&bytecode);

            /*
            use codec::Encode;
            let keccak = ink_utils::hash::keccak256("validate".as_bytes());
            let selector = [keccak[3], keccak[2], keccak[1], keccak[0]];
            let mut call = selector.encode();
            call.append( &mut Encode::encode(&registry_id) );
            */

            let addr = <NftRegistryTest as contracts::Trait>::DetermineContractAddress::contract_address_for(
                &codehash,
                &codec::Encode::encode(&ALICE),
                &ALICE);

            println!("Contract address: {:?}", addr);

            assert_ok!(
                NftReg::new_registry(origin.clone(), addr)
            );

            println!("Call: {:?}", call);

            assert_ok!(
                NftReg::mint(origin, registry_id, call.to_bytes().to_vec(), 0, 100_000)
            );
            /*
            let res = <contracts::Module<NftRegistryTest>>::bare_call(
                ALICE,
                addr,
                0,
                100_000,
                codec::Encode::encode(&call));
                //call.to_bytes().to_vec());
            */

            //println!("Call result: {:?}", res.ok().map(|r| r.data));
            /*
            println!("Call: {:?}", call);
            assert_ok!(
                Contract::call(
                    Origin::signed(ALICE),
                    addr,
                    0,
                    100_000,
                    //call)
                    call.to_bytes().to_vec())
                    //selector.to_vec())
            );
            */

            println!("Event log:");
            for e in &<system::Module<NftRegistryTest>>::events() {
                println!("{:?}", e);
            }
        });
    }
}
