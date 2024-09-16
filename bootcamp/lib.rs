#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod bootcamp {
    use crate::bootcamp::Error::Overflow;
    use ink::prelude::vec::Vec;
    use ink::storage::{Lazy, Mapping};

    #[ink(storage)]
    pub struct Bootcamp {
        value: u8,
        transferred: Balance,
        old_values: Vec<u8>,
        admin: AccountId,
        sort_of_mapping: Mapping<AccountId, u8>,
        lazy_value: Lazy<u8>,
    }

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        Overflow,
    }

    impl Bootcamp {
        #[ink(constructor)]
        pub fn new(init_value: u8) -> Self {
            Self {
                value: init_value,
                transferred: 0,
                old_values: Vec::new(),
                admin: Self::env().caller(),
                sort_of_mapping: Mapping::new(),
                lazy_value: Lazy::new(),
            }
        }

        #[ink(message)]
        pub fn inc(&mut self) -> Result<(), Error> {
            self.old_values.push(self.value);
            if self.value.checked_add(1) == None {
                Err(Overflow)
            } else {
                Ok(())
            }
        }

        #[ink(message)]
        pub fn add_all(&mut self) -> Result<(), Error> {
            let sum: u128 = 0;
            for value in self.old_values.iter() {
                match sum.checked_add(*value as u128) {
                    None => {
                        return Err(Overflow);
                    }
                    Some(_) => {}
                };
            }
            Ok(())
        }

        #[ink(message)]
        pub fn pay_to_be_admin(&mut self) -> Result<(), Error> {
            if self.env().transferred_value() > 100 {
                self.admin = self.env().caller();
            }
            Ok(())
        }

        #[ink(message)]
        pub fn changes_admin(&mut self, admin: AccountId) {
            self.admin = admin;
        }

        #[ink(message)]
        pub fn update_mapping(&mut self, acc: AccountId) {
            self.sort_of_mapping.insert(acc, &self.value);
        }

        #[ink(message)]
        pub fn add_to_lazy_val(&self, number: u8) -> Result<(), Error> {
            let mut val = self.lazy_value.get().unwrap_or_default();
            match val.checked_add(number) {
                None => Err(Overflow),
                Some(_) => Ok(()),
            }
        }
    }
}
