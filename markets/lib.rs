#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod markets {

    use erc721::Erc721Ref;
    use trait_erc20::TErc20;

    #[ink(storage)]
    pub struct Markets {
        acceptable_erc20: ink::contract_ref!(TErc20),
        erc721: Erc721Ref,
        price: Balance,
        minted_count: u32,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Erc20TransferFailed,
        Erc721MintFailed,
        Erc721TransferFailed,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    impl Markets {
        #[ink(constructor)]
        pub fn new(erc721: Erc721Ref, erc20: AccountId) -> Self {
            Self {
                acceptable_erc20: erc20.into(),
                erc721,
                price: 20,
                minted_count: 0,
            }
        }

        #[ink(message)]
        pub fn buy_nft(&mut self) -> Result<()> {
            let sender = self.env().caller();
            let transfer_erc20_to_market_res =
                self.acceptable_erc20
                    .transfer_from(sender, self.env().account_id(), self.price);

            if transfer_erc20_to_market_res.is_err() {
                return Err(Error::Erc20TransferFailed);
            }

            self.minted_count += 1;
            let mint_res = self.erc721.mint(self.minted_count);
            if mint_res.is_err() {
                return Err(Error::Erc721MintFailed);
            }

            // 由于mint的调用者是合约，mint之后的erc721是属于合约的，所以需要将erc721转给购买者
            let transfer_erc721_to_buyer_res = self.erc721.transfer(sender, self.minted_count);

            if transfer_erc721_to_buyer_res.is_err() {
                return Err(Error::Erc721TransferFailed);
            }

            Ok(())
        }
    }
}
