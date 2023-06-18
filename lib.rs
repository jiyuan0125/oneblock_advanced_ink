#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod erc20 {
    use ink::storage::Mapping;
    use trait_erc20::{TErc20, Error, Result};

    #[ink(storage)]
    #[derive(Default)]
    pub struct Erc20 {
        total_supply: Balance,
        balances: Mapping<AccountId, Balance>,
        allowances: Mapping<(AccountId, AccountId), Balance>,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        value: Balance,
    }

    impl Erc20 {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(total_supply: Balance) -> Self {
            let mut balances = Mapping::new();
            balances.insert(&Self::env().caller(), &total_supply);

            Self::env().emit_event(Transfer {
                from: None,
                to: Some(Self::env().caller()),
                value: total_supply,
            });

            Self {
                total_supply,
                balances,
                ..Default::default()
            }
        }


        pub fn transfer_helper(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            value: Balance,
        ) -> Result<()> {
            let balance_from = self.balance_of(*from);
            let balance_to = self.balance_of(*to);

            if value > balance_from {
                return Err(Error::BalanceTooLow);
            }

            self.balances.insert(from, &(balance_from - value));
            self.balances.insert(to, &(balance_to + value));

            self.env().emit_event(Transfer {
                from: Some(*from),
                to: Some(*to),
                value,
            });

            Ok(())
        }
    }

    impl TErc20 for Erc20 {
        #[ink(message)]
        fn total_supply(&self) -> Balance {
            self.total_supply
        }

        #[ink(message)]
        fn balance_of(&self, who: AccountId) -> Balance {
            self.balances.get(&who).unwrap_or_default()
        }

        #[ink(message)]
        fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            self.transfer_helper(&self.env().caller(), &to, value)
        }

        #[ink(message)]
        fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: Balance,
        ) -> Result<()> {
            let sender = self.env().caller();
            let mut its_allowance = self.allowances.get(&(from, sender)).unwrap_or_default();
            if its_allowance < value {
                return Err(Error::AllowanceTooLow);
            }

            self.transfer_helper(&from, &to, value)?;
            its_allowance -= value;
            self.allowances.insert(&(from, sender), &its_allowance);

            Ok(())
        }

        #[ink(message)]
        fn approve(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let sender = self.env().caller();
            self.allowances.insert(&(sender, to), &value);

            self.env().emit_event(Approval {
                from: sender,
                to,
                value,
            });

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        type Event = <Erc20 as ::ink::reflect::ContractEventBase>::Type;

        #[ink::test]
        fn constructor_works() {
            let erc20 = Erc20::new(10000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            assert_eq!(erc20.total_supply(), 10000);
            assert_eq!(erc20.balance_of(accounts.alice), 10000);

            let emmited_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            let event = &emmited_events[0];
            let decoded = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("failed to decode event data");

            match decoded {
                Event::Transfer(Transfer { from, to, value }) => {
                    assert!(from.is_none());
                    assert_eq!(to, Some(accounts.alice), "minted to wrong account");
                    assert_eq!(value, 10000, "minted wrong amount");
                }
                _ => panic!("match error"),
            }
        }

        #[ink::test]
        fn transfer_works() {
            let mut erc20 = Erc20::new(10000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let res = erc20.transfer(accounts.bob, 1000);
            assert!(res.is_ok());
            assert_eq!(erc20.balance_of(accounts.alice), 10000 - 1000);
            assert_eq!(erc20.balance_of(accounts.bob), 1000);

            let emmited_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            let event = &emmited_events[1];
            let decoded = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("failed to decode event data");

            match decoded {
                Event::Transfer(Transfer { from, to, value }) => {
                    assert_eq!(from, Some(accounts.alice), "transfered from wrong account");
                    assert_eq!(to, Some(accounts.bob), "transfered to wrong account");
                    assert_eq!(value, 1000, "transfered wrong amount");
                }
                _ => panic!("match error"),
            }
        }

        #[ink::test]
        fn invalid_transfer_fails() {
            let mut erc20 = Erc20::new(10000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let res = erc20.transfer(accounts.charlie, 1000);
            assert!(res.is_err());
            assert_eq!(res, Err(Error::BalanceTooLow));
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {

        use super::*;

        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test]
        async fn e2e_transfer(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            let total_supply = 10000;
            // 这里的Erc20Ref是一个包装器，用来包装合约的地址，方便调用合约
            // 这是魔法，通过展开 ink::contract 宏可以看到细节
            let constructor = Erc20Ref::new(total_supply);

            let contract_acc_id = client
                .instantiate("erc20", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("failed to deploy contract")
                .account_id;

            let alice_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);

            let transfer_msg = ink_e2e::build_message::<Erc20Ref>(contract_acc_id.clone())
                .call(|erc20| erc20.transfer(bob_acc, 1000));

            let res = client.call(&ink_e2e::alice(), transfer_msg, 0, None).await;
            assert!(res.is_ok());

            let balance_msg = ink_e2e::build_message::<Erc20Ref>(contract_acc_id.clone())
                .call(|erc20| erc20.balance_of(alice_acc));
            let balance_of_alice = client.call_dry_run(&ink_e2e::alice(), &balance_msg, 0, None).await;

            assert_eq!(balance_of_alice.return_value(), 9000);

            Ok(())
        }
    }
}
