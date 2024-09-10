#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod investment_fund {
    use ink::{
        env::{
            call::{build_call, ExecutionInput, Selector},
            CallFlags, DefaultEnvironment,
        },
        storage::{traits::ManualKey, Lazy, Mapping},
    };
    use ink::codegen::Env;
    use ink::primitives::Hash;

    #[ink(storage)]
    pub struct InvestmentFund {
        users: Mapping<AccountId, Balance, ManualKey<0xCF>>,
        strategy: Lazy<Hash>,
        manager: AccountId,
    }

    impl InvestmentFund {
        #[ink(constructor)]
        pub fn new(init_value: i32, hash: Hash) -> Self {
            let v = Mapping::new();

            let mut strategy = Lazy::new();
            strategy.set(&hash);
            Self::env().lock_delegate_dependency(&hash);

            Self {
                users: v,
                strategy,
                manager: Self::env().caller()
            }
        }

        /// Update the hash of the contract to delegate to.
        /// - Unlocks the old delegate dependency, releasing the deposit and allowing old
        ///   delegated to code to be removed.
        /// - Adds a new delegate dependency lock, ensuring that the new delegated to code
        ///   cannot be removed.
        #[ink(message)]
        pub fn update_strategy(&mut self, hash: Hash) {
            self.caller_is_manager();

            if let Some(old_hash) = self.strategy.get() {
                self.env().unlock_delegate_dependency(&old_hash)
            }
            self.env().lock_delegate_dependency(&hash);
            self.strategy.set(&hash);
        }

        /// Increment the current value using delegate call.
        #[ink(message)]
        pub fn invest_in_strategy(&mut self) {
            self.caller_is_manager();

            let selector = ink::selector_bytes!("activate");
            let _ = build_call::<DefaultEnvironment>()
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
                .try_invoke();
        }

        #[ink(message)]
        pub fn how_much_deposited(&self) -> Balance {
            self.users.get(&self.env().caller()).unwrap_or(0)
        }

        #[ink(message, payable)]
        pub fn deposit(&mut self) {
            let caller = self.env().caller();
            let amount = self.env().transferred_value();
            let balance = self.users.get(&caller).unwrap_or(0);
            self.users.insert(caller, &(balance.saturating_add(amount)));
        }

        #[ink(message)]
        pub fn calculate_shares(&self) -> Balance {
            let total_deposits: Balance = self.users.values().sum();
            let caller = self.env().caller();
            let balance = self.users.get(&caller).unwrap_or(0);
            balance * 100 / total_deposits
        }

        #[ink(message)]
        pub fn calculate_tokens(&self) -> Balance {
            let total_deposits: Balance = self.env().balance();
            let caller = self.env().caller();
            let balance = self.users.get(&caller).unwrap_or(0);
            balance * 100 / total_deposits
        }


        fn strategy(&self) -> Hash {
            self.strategy.get().expect("strategy always has a value")
        }

        fn caller_is_manager(&self) {
            assert_eq!(self.env().caller(), self.manager, "caller is not the manager");
        }



    }
}
