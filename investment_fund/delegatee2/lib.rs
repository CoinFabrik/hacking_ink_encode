#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod delegatee2 {
    use ink::storage::{
        traits::ManualKey,
        Mapping,
    };
    #[ink(storage)]
    pub struct Delegatee2 {
        addresses: Mapping<AccountId, i32, ManualKey<0x23>>,
        counter: i32,
    }

    impl Delegatee2 {
        #[ink(constructor)]
        pub fn new() -> Self {
            unreachable!(
                "Constructors are not called when upgrading using `set_code_hash`."
            )
        }

        #[ink(message)]
        pub fn activate(&mut self) {
            todo!()
        }

    }
}
