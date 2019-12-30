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
//use runtime_primitives::traits::{As, Hash};
use sp_runtime::traits::{StaticLookup};
//use parity_codec::{Encode, Decode};
use system::{ensure_signed, RawOrigin};
//use runtime_primitives::traits::StaticLookup;
//use crate::Event;

//use crate::wasm::ExecutionContext;

use sp_std::prelude::*;

struct NftContract;

// A unique id for a NFT type (not an NFT instance)
//type RegistryUid<T: Trait> = T::Hash;
type RegistryUid = u64;

pub trait Trait: balances::Trait + contracts::Trait  {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event! {
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash,
    {
        NewRegistry(AccountId, RegistryUid),
        Mint(AccountId, Hash),
    }
}

decl_storage!{
    trait Store for Module<T: Trait> as Nfttest {
        ValidationFn get(validator_of): map RegistryUid => Option<T::AccountId>;
        // TODO: need a generic enough type to represent any possible contract
        // NftRegistry get(contract_of): map RegistryUid<T> => 
        //NftInstance: map T::Hash => NftContract;
        Nonce: u64;
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

            // Mint the nft
            //<Contract<T>::insert(uid, contract);

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use support::{impl_outer_origin, impl_outer_event, impl_outer_dispatch, assert_ok};
    use runtime_io::{with_externalities, TestExternalities};
    use primitives::{H256, Blake2Hasher, sr25519};
    use runtime_primitives::{
        BuildStorage, traits::{BlakeTwo256, IdentityLookup},
        testing::{Digest, DigestItem, Header}
    };

    mod nftregistry {
        // Re-export contents of the root. This basically
        // needs to give a name for the current crate.
        // This hack is required for `impl_outer_event!`.
        pub use super::super::*;
        use support::impl_outer_event;
    }

    #[derive(Eq, Clone, PartialEq)]
    pub struct NftRegistryTest;

    impl_outer_origin! {
        pub enum Origin for NftRegistryTest {}
    }

    impl_outer_event! {
        pub enum MetaEvent for NftRegistryTest {
            balances<T>, contracts<T>, nftregistry<T>,
        }
    }

    /*
    use crate::{Balances, Contract, NftRegistry};
    impl_outer_dispatch! {
        pub enum Call for Test where origin: Origin {
            balances::Balances,
            contract::Contract,
            nftregistry::NftRegistry,
        }
    }
    */

    impl system::Trait for NftRegistryTest {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Digest = Digest;
        type AccountId = super::super::AccountId;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = MetaEvent;
        type Log = DigestItem;
    }

    impl balances::Trait for NftRegistryTest {
        type Balance = u64;
        type OnFreeBalanceZero = ();
        type OnNewAccount = ();
        type Event = MetaEvent;
        type TransactionPayment = ();
        type TransferPayment = ();
        type DustRemoval = ();
    }

    impl timestamp::Trait for NftRegistryTest {
        type Moment = u64;
        type OnTimestampSet = ();
    }

    impl contracts::Trait for NftRegistryTest {
        type Currency = crate::Balances;
        type Call = contracts::Call<NftRegistryTest>;
        type Event = MetaEvent;
        type Gas = u64;
        type DetermineContractAddress = contracts::SimpleAddressDeterminator<NftRegistryTest>;
        type ComputeDispatchFee = contracts::DefaultDispatchFeeComputor<NftRegistryTest>;
        type TrieIdGenerator = contracts::TrieIdFromParentCounter<NftRegistryTest>;
        type GasPayment = ();
    }

    impl super::Trait for NftRegistryTest {
        type Event = MetaEvent;
    }

    type NftReg = super::Module<NftRegistryTest>;

    fn build_ext() -> TestExternalities<Blake2Hasher> {
        let mut t = system::GenesisConfig::<NftRegistryTest>::default().build_storage().unwrap().0;
        t.extend(balances::GenesisConfig::<NftRegistryTest>::default().build_storage().unwrap().0);

        let h1 = sr25519::Public::from_h256((1).using_encoded(<NftRegistryTest as system::Trait>::Hashing::hash));
        let h2 = sr25519::Public::from_h256((2).using_encoded(<NftRegistryTest as system::Trait>::Hashing::hash));
        let endowed_accounts = vec![h1, h2];

        t.extend(balances::GenesisConfig::<NftRegistryTest> {
            transaction_base_fee: 1,
            transaction_byte_fee: 0,
            existential_deposit: 500,
            transfer_fee: 0,
            creation_fee: 0,
            balances: endowed_accounts.iter().cloned().map(|k|(k, 1 << 60)).collect(),
            vesting: vec![],
        }.build_storage().unwrap().0);
        t.extend(contracts::GenesisConfig::<NftRegistryTest>::default().build_storage().unwrap().0);
        t.into()
    }

    fn init_contract(origin: Origin) -> Result {
        use std::{io, io::prelude::*, fs::File};
        use std::path::Path;

        // Get the wasm contract byte code from a file
        let mut f = File::open(Path::new("./test_wasm/testcontract.wasm"))
            .map_err(|_| "Failed to open contract file")?;
        let mut bytecode = Vec::<u8>::new();
        f.read_to_end(&mut bytecode)
            .map_err(|_| "Didn't read to end of file")?;

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

        // Initialize as contract
        <contracts::Module<NftRegistryTest>>::create(origin, 0, 200_000, codehash, vec![])
    }

    #[test]
    fn create_nft_registry() {
        with_externalities(&mut build_ext(), || {
            let h1 = sr25519::Public::from_h256((1).using_encoded(<NftRegistryTest as system::Trait>::Hashing::hash));
            let h2 = sr25519::Public::from_h256((2).using_encoded(<NftRegistryTest as system::Trait>::Hashing::hash));
            let origin = Origin::signed(h1.clone());

            println!("Free bal: {}", <balances::Module<NftRegistryTest>>::free_balance(&h1.clone()));

            //init_contract( origin.clone() );

            assert_ok!(
                NftReg::new_registry(origin,h2)
            );

            println!("Event log:");
            for e in &<system::Module<NftRegistryTest>>::events() {
                println!("{:?}", e);
            }
        });
    }
}
