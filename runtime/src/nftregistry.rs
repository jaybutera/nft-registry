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

        /*
        fn upload_validation_contract(
            origin,
            gas: T::Gas) -> Result
        {
            let sender = ensure_signed(origin)?;

            // Store bytecode
            let codehash = <contract::Module<T>>::put_code(
                T::Origin::from(RawOrigin::<T::AccountId>::Signed(sender.clone())),
                gas,
                //<T::Gas as As<u32>>::sa(210000),
                bytecode)?;

            // Initialize contract
            <contract::Module<T>>::create(
                T::Origin::from(RawOrigin::<T::AccountId>::Signed(sender.clone())),
                value,
                gas,
                codehash,
                data)?;
        }
        */

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
                // Origin::signed( sender.clone() ),
                T::Origin::from(RawOrigin::<T::AccountId>::Signed(sender.clone())),
                T::Lookup::unlookup(validation_fn.clone()),
                value,
                gas_limit,
                parameters)?;

            Ok(())
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

    fn get_wasm_bytecode() -> std::result::Result<Vec<u8>, &'static str> {
        use std::{io, io::prelude::*, fs::File};
        use std::path::Path;

        // Get the wasm contract byte code from a file
        let mut f = File::open(Path::new("./testcontract.wasm"))
            .map_err(|_| "Failed to open contract file")?;
        let mut bytecode = Vec::<u8>::new();
        f.read_to_end(&mut bytecode)
            .map(|_| bytecode)
            .map_err(|_| "Didn't read to end of file")
    }

    fn init_contract(origin: Origin) -> Result {
        let bytecode = get_wasm_bytecode()?;
        //let codehash = (bytecode).using_encoded(<T as system::Trait>::Hashing::hash);

        // Store code on chain
        <contracts::Module<NftRegistryTest>>::put_code(
            origin.clone(),
            100_000,
            bytecode
            //0x14144020u32.to_le_bytes().to_vec()
            )?;

        // Get codehash from event log
        let codehash_event = <system::Module<NftRegistryTest>>::events().pop()
            .ok_or("An event should be in the log but its not")?;
        let codehash = match codehash_event.event {
            MetaEvent::contracts(contracts::RawEvent::CodeStored(hash)) => Some(hash),
            _ => None,
        }.ok_or("Latest event is not a CodeStored event")?;

        println!("codehash: {:?}", codehash.clone());
        // Initialize as contract
        <contracts::Module<NftRegistryTest>>::instantiate(origin, 1_000, 100_000, codehash, vec![])

        /*
        let keccak = ink_utils::hash::keccak256("validate".as_bytes());
        let selector = [keccak[3], keccak[2], keccak[1], keccak[0]];
        //let s = codec::Encode::encode(&String::from("fn validate(&mut self, nft_class_id: &AccountId)"));
        //let selector = <NftRegistryTest as system::Trait>::Hashing::hash(&s.as_slice())[0..3];

        Contract::call(
            Origin::signed(ALICE),
            BOB,
            0,
            100_000_000,
            selector.to_vec())
            */
    }

    #[test]
    fn create_nft_registry() {
        ExtBuilder::default().build().execute_with(|| {
            Balances::deposit_creating(&ALICE, 800_000_000_000);
            let origin = Origin::signed(ALICE);

            println!("Free bal: {}", <balances::Module<NftRegistryTest>>::free_balance(&ALICE));

            assert_ok!(
                init_contract( origin.clone() )
            );

            assert_ok!(
                NftReg::new_registry(origin,BOB)
            );

            println!("Event log:");
            for e in &<system::Module<NftRegistryTest>>::events() {
                println!("{:?}", e);
            }
        });
    }
}
