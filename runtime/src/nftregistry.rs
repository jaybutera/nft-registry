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

            // TODO: How to determine if validation succeeds?
            <contract::Module<T>>::call(
                T::Origin::from(RawOrigin::<T::AccountId>::Signed(sender)),
                T::Lookup::unlookup(validation_fn),
                value,
                gas_limit,
                parameters)?;

            // Create NFT if validation succeeds

            // Emit event
            Self::deposit_event(RawEvent::Mint(sender, uid));

            Ok(())
        }
    }
}
