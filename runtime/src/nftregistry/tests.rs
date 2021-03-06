use nftregistry::*;
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

const CODE_DISPATCH_CALL: &str = r#"
(module
    (import "env" "ext_dispatch_call" (func $ext_dispatch_call (param i32 i32)))
    (import "env" "memory" (memory 1 1))
    (func (export "call")
        (call $ext_dispatch_call
            (i32.const 8) ;; Pointer to the start of encoded call buffer
            (i32.const 10) ;; Length of the buffer
        )
    )
    (func (export "deploy"))
    ;; Generated by codec::Encode::encode(&Call::NftRegistry(nftregistry::Call::finish_mint(registry_id))))
    (data (i32.const 8) "\02\02\00\00\00\00\00\00\00\00")
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

    // Get the wasm contract byte code from a file
    let mut f = File::open(Path::new("./testcontract.wasm"))
        .map_err(|_| "Failed to open contract file")?;
    let mut bytecode = Vec::<u8>::new();
    f.read_to_end(&mut bytecode)
        .map(|_| bytecode)
        .map_err(|_| "Didn't read to end of file")
}

#[test]
fn mint_nft_from_basic_contract () {
    ExtBuilder::default().build().execute_with(|| {
        Balances::deposit_creating(&ALICE, 100_000_000);
        let origin = Origin::signed(ALICE);
        let registry_id = 0;

        // Compile wasm for deployment
        let (bytecode, codehash) = compile_module::<NftRegistryTest>(CODE_DISPATCH_CALL).unwrap();

        // Store code on chain
        assert_ok!(
            <contracts::Module<NftRegistryTest>>::put_code(
                origin.clone(),
                100_000,
                bytecode
                )
            .and_then(|_|
            <contracts::Module<NftRegistryTest>>::instantiate(
                origin.clone(),
                1_000,
                100_000,
                codehash,
                codec::Encode::encode(&ALICE)))
        );

        // Determine the address the contract was assigned
        let contract_addr = <NftRegistryTest as contracts::Trait>::DetermineContractAddress::contract_address_for(
            &codehash,
            &codec::Encode::encode(&ALICE),
            &ALICE);

        // Create registry and mint nft
        assert_ok!(
            NftReg::new_registry(origin.clone(), contract_addr)
                .and_then(|_|
            NftReg::mint(origin, registry_id, vec![], 0, 100_000))
        );

        // Check event logs to see that nft was minted
        assert!(
            <system::Module<NftRegistryTest>>::events().iter()
                .find(|e| match e.event {
                    MetaEvent::nftregistry(RawEvent::MintNft(_,_)) => true,
                    _ => false,
                })
            .is_some()
        );
    });
}

#[test]
fn mint_nft_from_ink_contract() {
    ExtBuilder::default().build().execute_with(|| {
        Balances::deposit_creating(&ALICE, 100_000_000);
        let origin = Origin::signed(ALICE);
        let registry_id = 0;

        // Get wasm bytecode
        let bytecode = get_wasm_bytecode().unwrap();
        let codehash = <NftRegistryTest as system::Trait>::Hashing::hash(&bytecode);

        // Store and instantiate contract
        assert_ok!(
            <contracts::Module<NftRegistryTest>>::put_code(
                origin.clone(),
                100_000,
                bytecode
                ).and_then(|_|
            <contracts::Module<NftRegistryTest>>::instantiate(origin.clone(), 1_000, 100_000, codehash, codec::Encode::encode(&ALICE)))
        );

        // Call validation contract method
        //let mut call = CallData::new( Selector::from_str("validate") );
        //let mut call = CallData::new( Selector::from_str("finish_mint") );
        //call.push_arg(&codec::Encode::encode(&2));
        //call.push_arg(&2);
        //call.push_arg(&registry_id);

        //let encoded = Encode::encode(&Call::Balances(pallet_balances::Call::transfer(CHARLIE, 50)));
        println!("ENCODED: {:?}", codec::Encode::encode(&Call::NftRegistry(nftregistry::Call::finish_mint(registry_id))));
        let call = codec::Encode::encode(&Call::NftRegistry(nftregistry::Call::finish_mint(registry_id)));

        /*
        use codec::Encode;
        let keccak = ink_utils::hash::keccak256("finish_mint".as_bytes());
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

        //println!("Call: {:?}", call);

        assert_ok!(
            //NftReg::mint(origin, registry_id, codec::Encode::encode(&call.to_bytes().to_vec()), 0, 100_000)
            NftReg::mint(origin, registry_id, vec![], 0, 100_000)
        );
        /*
        let res = <contracts::Module<NftRegistryTest>>::bare_call(
            ALICE,
            addr,
            0,
            100_000,
            call);
            //codec::Encode::encode(&call.to_bytes().to_vec()));
            //call.to_bytes().to_vec());
            //vec![]);
        */

        //println!("Call result: {:?}", res.ok().map(|r| (r.is_success(), r.data)));
        //println!("Call result: {:?}", res.err().map(|e| (e.reason, e.buffer)));
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
