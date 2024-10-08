#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod investment_fund {
    use crate::investment_fund::Error::{ArithmeticError, InvokeError, NotEnoughShares};
    use ink::codegen::Env;
    use ink::{
        env::{
            call::{build_call, ExecutionInput, Selector},
            CallFlags, DefaultEnvironment,
        },
        storage::{traits::ManualKey, Lazy, Mapping},
    };

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        ArithmeticError,
        NotEnoughShares,
        InvokeError,
    }

    #[ink(storage)]
    pub struct InvestmentFund {
        users: Mapping<AccountId, Balance, ManualKey<0xCF>>,
        strategy: Lazy<Hash, ManualKey<0xCFCF>>,
        manager: AccountId,
        users_total_shares: u128,
        fee: u128,
    }

    impl InvestmentFund {
        #[ink(constructor)]
        pub fn new(hash: Hash, fee: u128) -> Self {
            let v = Mapping::new();

            let mut strategy = Lazy::new();
            strategy.set(&hash);
            Self::env().lock_delegate_dependency(&hash);

            Self {
                users: v,
                strategy,
                manager: Self::env().caller(),
                users_total_shares: 0,
                fee,
            }
        }
        #[ink(message)]
        pub fn update_strategy(&mut self, hash: Hash) {
            self.caller_is_manager();

            if let Some(old_hash) = self.strategy.get() {
                self.env().unlock_delegate_dependency(&old_hash)
            }
            self.env().lock_delegate_dependency(&hash);
            self.strategy.set(&hash);
        }

        #[ink(message)]
        pub fn invest_in_strategy(&mut self) -> Result<(), Error> {
            self.caller_is_manager();

            let selector = ink::selector_bytes!("activate");
            match build_call::<DefaultEnvironment>()
                .delegate(self.strategy())
                // We specify `CallFlags::TAIL_CALL` to use the delegatee last memory frame
                // as the end of the execution cycle.
                // So any mutations to `Packed` types, made by delegatee,
                // will be flushed to storage.
                //
                // If we don't specify this flag.
                // The storage state before the delegate call will be flushed to storage instead.
                // See https://substrate.stackexchange.com/questions/3336/i-found-set-allow-reentry-may-have-some-problems/3352#3352
                .call_flags(CallFlags::TAIL_CALL)
                .exec_input(ExecutionInput::new(Selector::new(selector)))
                .returns::<()>()
                .try_invoke()
            {
                Ok(_) => Ok(()),
                Err(_) => Err(InvokeError.into()),
            }
        }

        #[ink(message)]
        pub fn get_shares(&self) -> Balance {
            self.users.get(self.env().caller()).unwrap_or(0)
        }

        #[ink(message, payable)]
        pub fn deposit(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let amount = self.env().transferred_value();
            let shares = self.users.get(caller).unwrap_or(0);
            let new_shares = match self.calculate_shares(amount) {
                Ok(v) => v,
                Err(e) => return Err(e.into()),
            };
            match self.users_total_shares.checked_add(new_shares) {
                Some(v) => self.users_total_shares = v,
                None => return Err(ArithmeticError.into()),
            }
            self.users.insert(
                caller,
                &(shares.checked_add(new_shares).unwrap_or_default()),
            );
            Ok(())
        }

        #[ink(message)]
        pub fn withdraw(&mut self, amount: Balance) -> Result<(), Error> {
            let caller = self.env().caller();
            let shares = self.users.get(caller).unwrap_or_default();

            if shares < amount {
                return Err(NotEnoughShares.into());
            }

            self.users
                .insert(caller, &(shares.checked_sub(amount).unwrap()));

            match self.users_total_shares.checked_sub(amount) {
                Some(v) => self.users_total_shares = v,
                None => return Err(NotEnoughShares.into()),
            };

            let Ok(removed_tokens) = self.calculate_tokens(amount) else {
                return Err(ArithmeticError.into());
            };

            let fee = removed_tokens
                .checked_mul(self.fee)
                .unwrap()
                .checked_div(100)
                .unwrap();

            // Ensure contract has enough balance to fulfill the withdrawal
            if self.env().balance() < removed_tokens {
                // Retrieve required tokens from strategy
                let selector = ink::selector_bytes!("retrieve_tokens");
                match build_call::<DefaultEnvironment>()
                    .delegate(self.strategy())
                    .exec_input(
                        ExecutionInput::new(Selector::new(selector)).push_arg(&removed_tokens),
                    )
                    .returns::<()>()
                    .try_invoke()
                {
                    Ok(_) => {}
                    Err(_) => {
                        return Err(InvokeError.into());
                    }
                }
            }
            self.env()
                .transfer(caller, removed_tokens.checked_sub(fee).unwrap())
                .expect("Transfer failed");
            self.env()
                .transfer(self.manager, fee)
                .expect("Transfer failed");

            Ok(())
        }

        #[ink(message)]
        pub fn calculate_shares(&mut self, amount: Balance) -> Result<Balance, Error> {
            let total_shares: Balance = self.users_total_shares;
            if total_shares == 0 {
                Ok(amount)
            } else {
                let selector = ink::selector_bytes!("get_balance");
                let strategy_balance: Balance = build_call::<DefaultEnvironment>()
                    .delegate(self.strategy())
                    .exec_input(ExecutionInput::new(Selector::new(selector)))
                    .returns::<Balance>()
                    .invoke();
                match amount.checked_mul(total_shares) {
                    Some(v) => match v.checked_div(strategy_balance) {
                        Some(v) => Ok(v),
                        None => return Err(ArithmeticError.into()),
                    },
                    None => return Err(ArithmeticError.into()),
                }
            }
        }

        #[ink(message)]
        pub fn calculate_tokens(&self, shares: Balance) -> Result<Balance, Error> {
            let total_shares: Balance = self.users_total_shares;
            let selector = ink::selector_bytes!("get_balance");
            let strategy_balance: Balance = match build_call::<DefaultEnvironment>()
                .delegate(self.strategy())
                .exec_input(ExecutionInput::new(Selector::new(selector)))
                .returns::<Balance>()
                .try_invoke()
            {
                Ok(v) => v.unwrap(),
                Err(_) => return Err(InvokeError.into()),
            };

            match shares.checked_mul(strategy_balance) {
                Some(v) => Ok(v.checked_div(total_shares).unwrap_or_default()),
                None => Err(ArithmeticError.into()),
            }
        }

        fn strategy(&self) -> Hash {
            self.strategy.get().expect("strategy always has a value")
        }

        fn caller_is_manager(&self) {
            assert_eq!(
                self.env().caller(),
                self.manager,
                "caller is not the manager"
            );
        }
    }
}
