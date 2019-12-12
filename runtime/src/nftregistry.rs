/// A runtime module template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references


/// For more guidance on Substrate modules, see the example module
/// https://github.com/paritytech/substrate/blob/master/srml/example/src/lib.rs

use support::{
    decl_module,
    ensure,
    decl_storage,
    decl_event,
    StorageValue,
    StorageMap,
    traits::Currency,
    dispatch::Result};
use runtime_primitives::traits::{As, Hash};
use parity_codec::{Encode, Decode};
use system::{ensure_signed, RawOrigin};
use runtime_primitives::traits::StaticLookup;

//use crate::wasm::ExecutionContext;

use rstd::prelude::*;

pub trait Trait: balances::Trait + contract::Trait  {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event! {
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash,
    {
        Created(AccountId, Hash),
        Mint(AccountId, Hash),
    }
}

decl_storage!{
    trait Store for Module<T: Trait> as Nfttest {
        ValidationFn get(validator_of): map T::Hash => Option<T::AccountId>;
        Nonce: u64;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        fn new_registry(origin, validation_fn_addr: T::AccountId) -> Result {
            let sender = ensure_signed(origin)?;

            // Get some randomness
            let rnd_seed = <system::Module<T>>::random_seed();
            let nonce = <Nonce<T>>::get();

            // Generate a uid and check that it's unique
            let uid = (rnd_seed, sender.clone(), nonce).using_encoded(<T as system::Trait>::Hashing::hash);
            ensure!(!<ValidationFn<T>>::exists(uid), "This new id for a registry already exists!");

            // Check for overflow on index
            let nplus1 = <Nonce<T>>::get().checked_add(1)
                .ok_or("Nonce overflow when adding a new registry")?;

            // Write state
            <ValidationFn<T>>::insert(&uid, validation_fn_addr);
            <Nonce<T>>::put( nplus1 );

            // Events
            Self::deposit_event(RawEvent::Created(sender, uid));

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
                uid: T::Hash,
                parameters: Vec<u8>,            // To be passed into the smart contract
                value: contract::BalanceOf<T>,  // If currency needs to be passed to contract
                gas_limit: T::Gas) -> Result
        {
            // TODO: Needs to ensure signed before anything else
            let sender = ensure_signed(origin)?;

            ensure!(<ValidationFn<T>>::exists(uid), "No registry with this uid exists");

            // Run merkle validation

            // Run custom validation
            let validation_fn = Self::validator_of(&uid)
                .ok_or("This should not happen bcs ensure above^")?;

            // Wasm contract should emit an event for success or failure
            <contract::Module<T>>::call(
                T::Origin::from(RawOrigin::<T::AccountId>::Signed(sender.clone())),
                T::Lookup::unlookup(validation_fn),
                value,
                gas_limit,
                parameters)?;

            // Check event log to see if validation succeeded
            let events = <system::Module<T>>::events();

            // TODO: Iterate in reverse bcs event should be at or near the end
            /*
            events.filter(|e|
                match e.event {
                    RawEvent::ValidationSuccess(..) => ,
                    RawEvent::ValidationFailure(..) => ,
                    _ => false,
                }
            });
            */

            // Create NFT if validation succeeds

            // Emit event
            Self::deposit_event(RawEvent::Mint(sender, uid));

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use support::{impl_outer_origin, assert_ok};
    use runtime_io::{with_externalities, TestExternalities};
    use primitives::{H256, Blake2Hasher, sr25519};
    use runtime_primitives::{
        BuildStorage, traits::{BlakeTwo256, IdentityLookup},
        testing::{Digest, DigestItem, Header}
    };

    #[derive(Eq, Clone, PartialEq)]
    pub struct NftRegistryTest;

    impl_outer_origin! {
        pub enum Origin for NftRegistryTest {}
    }

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
        type Event = ();
        type Log = DigestItem;
    }

    impl balances::Trait for NftRegistryTest {
        type Balance = u64;
        type OnFreeBalanceZero = ();
        type OnNewAccount = ();
        type Event = ();
        type TransactionPayment = ();
        type TransferPayment = ();
        type DustRemoval = ();
    }

    impl timestamp::Trait for NftRegistryTest {
        type Moment = u64;
        type OnTimestampSet = ();
    }

    impl contract::Trait for NftRegistryTest {
        type Currency = crate::Balances;
        type Call = contract::Call<NftRegistryTest>;
        type Event = ();
        type Gas = u64;
        type DetermineContractAddress = contract::SimpleAddressDeterminator<NftRegistryTest>;
        type ComputeDispatchFee = contract::DefaultDispatchFeeComputor<NftRegistryTest>;
        type TrieIdGenerator = contract::TrieIdFromParentCounter<NftRegistryTest>;
        type GasPayment = ();
    }

    impl super::Trait for NftRegistryTest {
        type Event = ();
    }

    type NftReg = super::Module<NftRegistryTest>;

    fn build_ext() -> TestExternalities<Blake2Hasher> {
        let mut t = system::GenesisConfig::<NftRegistryTest>::default().build_storage().unwrap().0;
        t.extend(balances::GenesisConfig::<NftRegistryTest>::default().build_storage().unwrap().0);
        t.into()
    }

    #[test]
    fn create_nft_registry() {
        with_externalities(&mut build_ext(), || {
            let h1 = sr25519::Public::from_h256((1).using_encoded(<NftRegistryTest as system::Trait>::Hashing::hash));
            let h2 = sr25519::Public::from_h256((2).using_encoded(<NftRegistryTest as system::Trait>::Hashing::hash));

            assert_ok!(
                NftReg::new_registry(Origin::signed(h1),h2)
            );
        });
    }
}
