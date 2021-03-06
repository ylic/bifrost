// Copyright 2019-2020 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use core::convert::TryInto;
use frame_support::traits::{Get};
use frame_support::{Parameter, decl_module, decl_event, decl_error, decl_storage, ensure};
use sp_runtime::traits::{Member, AtLeast32Bit, Saturating, One, Zero, StaticLookup};
use sp_std::prelude::*;
use system::{ensure_signed, ensure_root};
use node_primitives::{
	AccountAsset, AssetRedeem, AssetTrait, AssetSymbol, FetchConvertPrice, Token, TokenPair, TokenPriceHandler, TokenType,
};

mod mock;
mod tests;

lazy_static::lazy_static! {
	pub static ref TOKEN_LIST: [Vec<u8>; 3] = {
		let dot = b"DOT".to_vec();
		let ksm = b"KSM".to_vec();
		let eos = b"EOS".to_vec();
		[dot, ksm, eos]
	};
	pub static ref VTOKEN_LIST: [Vec<u8>; 3] = {
		let vdot = b"vDOT".to_vec();
		let vksm = b"vKSM".to_vec();
		let veos = b"vEOS".to_vec();
		[vdot, vksm, veos]
	};
}

/// The module configuration trait.
pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + From<Self::Convert>;

	/// The units in which we record prices.
	type Price: Member + Parameter + Default + AtLeast32Bit + Copy + Zero;

	/// The units in which we record convert rate.
	type Convert: Member + Parameter + Default + AtLeast32Bit + Copy + Zero;

	/// The units in which we record costs.
	type Cost: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + From<Self::Balance>;

	/// The units in which we record incomes.
	type Income: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + From<Self::Balance>;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + Default + AtLeast32Bit + Copy;

	/// Handler for asset redeem
	type AssetRedeem: AssetRedeem<Self::AssetId, Self::AccountId, Self::Balance>;

	/// Handler for fetch convert rate from convert runtime
	type FetchConvertPrice: FetchConvertPrice<Self::AssetId, Self::Convert>;
}

decl_event! {
	pub enum Event<T>
		where <T as system::Trait>::AccountId,
			<T as Trait>::Balance,
			<T as Trait>::AssetId,
	{
		/// Some assets were created.
		Created(AssetId, TokenPair<Balance>),
		/// Some assets were issued.
		Issued(AssetId, TokenType, AccountId, Balance),
		/// Some assets were transferred.
		Transferred(AssetId, TokenType, AccountId, AccountId, Balance),
		/// Some assets were destroyed.
		Destroyed(AssetId, TokenType, AccountId, Balance),
		/// Bind Asset with AccountId
		AccountAssetCreated(AccountId, AssetId),
		/// Bind Asset with AccountId
		AccountAssetDestroy(AccountId, AssetId),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Token symbol is too long
		TokenSymbolTooLong,
		/// if Vec<u8> is empty, meaning this symbol is empty string
		EmptyTokenSymbol,
		/// Precision too big or too small
		InvalidPrecision,
		/// Asset id doesn't exist
		TokenNotExist,
		/// Transaction cannot be made if he amount of balances are 0
		ZeroAmountOfBalance,
		/// Amount of input should be less than or equal to origin balance
		InvalidBalanceForTransaction,
		/// Convert rate doesn't be set
		ConvertRateDoesNotSet,
		/// This is an invalid convert rate
		InvalidConvertRate,
		/// Vtoken id is not equal to token id
		InvalidTokenPair,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Assets {
		/// The number of units of assets held by any given asset ans given account.
		pub AccountAssets get(fn account_assets): map hasher(blake2_128_concat) (T::AssetId, TokenType, T::AccountId)
			=> AccountAsset<T::Balance, T::Cost, T::Income>;
		/// The number of units of prices held by any given asset.
		pub Prices get(fn prices) config(): map hasher(blake2_128_concat) (T::AssetId, TokenType) => T::Price;
		/// The next asset identifier up for grabs.
		pub NextAssetId get(fn next_asset_id) config(): T::AssetId;
		/// Details of the token corresponding to an asset id.
		pub Tokens get(fn token_details) config(): map hasher(blake2_128_concat) T::AssetId => TokenPair<T::Balance>;
		/// A collection of asset which an account owned
		pub AccountAssetIds get(fn account_asset_ids): map hasher(blake2_128_concat) T::AccountId => Vec<T::AssetId>;
	}
	add_extra_genesis {
		build(|config: &GenesisConfig<T>| {
			// initialze three assets id for these tokens
			<NextAssetId<T>>::put(config.next_asset_id);

			let token_precision = [4, 4, 4];
			let vtoken_precision = [8, 8, 8];
			for i in 0..=2 {
				// initialize token
				let token = Token::new(TOKEN_LIST[i as usize].clone(), token_precision[i as usize], 0.into());
				let vtoken = Token::new(VTOKEN_LIST[i as usize].clone(), vtoken_precision[i as usize], 0.into());
				<Tokens<T>>::insert(T::AssetId::from(i), TokenPair::new(token, vtoken));

				// initialize price
				<Prices<T>>::insert((T::AssetId::from(i), TokenType::Token), T::Price::from(0u32));
				<Prices<T>>::insert((T::AssetId::from(i), TokenType::VToken), T::Price::from(0u32));
			}
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Create a new class of fungible assets. It will have an
		/// identifier `AssetId` instance: this will be specified in the `Created` event.
		#[weight = T::DbWeight::get().writes(1)]
		pub fn create(origin, symbol: Vec<u8>, precision: u16) {
			ensure_root(origin)?;

			ensure!(!symbol.is_empty(), Error::<T>::EmptyTokenSymbol);
			ensure!(symbol.len() <= 32, Error::<T>::TokenSymbolTooLong);
			ensure!(precision <= 16, Error::<T>::InvalidPrecision);

			let (id, token_pair) = Self::asset_create(symbol, precision);

			Self::deposit_event(RawEvent::Created(id, token_pair));
		}

		/// Issue any amount of fungible assets.
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		pub fn issue(
			origin,
			id: AssetSymbol,
			token_type: TokenType,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance,
		) {
			ensure_root(origin)?;
			let id = T::AssetId::from(id as u32);

			ensure!(<Tokens<T>>::contains_key(id), Error::<T>::TokenNotExist);

			let target = T::Lookup::lookup(target)?;
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmountOfBalance);

			Self::asset_issue(id, token_type, target.clone(), amount);

			Self::deposit_event(RawEvent::Issued(id, token_type, target, amount));
		}

		/// Move some assets from one holder to another.
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		pub fn transfer(
			origin,
			id: AssetSymbol,
			token_type: TokenType,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance,
		) {
			let origin = ensure_signed(origin)?;
			let id = T::AssetId::from(id as u32);

			let origin_account = (id, token_type, origin.clone());
			let origin_balance = <AccountAssets<T>>::get(&origin_account).balance;
			let target = T::Lookup::lookup(target)?;

			ensure!(!amount.is_zero(), Error::<T>::ZeroAmountOfBalance);
			ensure!(origin_balance >= amount, Error::<T>::InvalidBalanceForTransaction);

			Self::asset_transfer(id, token_type, origin.clone(), target.clone(), amount);

			Self::deposit_event(RawEvent::Transferred(id, token_type, origin, target, amount));
		}

		/// Destroy any amount of assets of `id` owned by `origin`.
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		pub fn destroy(
			origin,
			id: AssetSymbol,
			token_type: TokenType,
			#[compact] amount: T::Balance,
		) {
			let origin = ensure_signed(origin)?;

			let id = T::AssetId::from(id as u32);
			let origin_account = (id, token_type, origin.clone());

			let balance = <AccountAssets<T>>::get(&origin_account).balance;
			ensure!(amount <= balance , Error::<T>::InvalidBalanceForTransaction);

			Self::asset_destroy(id, token_type, origin.clone(), amount);

			Self::deposit_event(RawEvent::Destroyed(id, token_type, origin, amount));
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		pub fn redeem(
			origin,
			id: AssetSymbol,
			token_type: TokenType,
			#[compact] amount: T::Balance,
			to_name: Option<Vec<u8>>,
		) {
			let origin = ensure_signed(origin)?;

			let id = T::AssetId::from(id as u32);
			let origin_account = (id, token_type, origin.clone());

			let balance = <AccountAssets<T>>::get(&origin_account).balance;
			ensure!(amount <= balance , Error::<T>::InvalidBalanceForTransaction);

			T::AssetRedeem::asset_redeem(id, token_type, origin.clone(), amount, to_name);

			Self::asset_destroy(id, token_type, origin, amount);
		}
	}
}

impl<T: Trait> AssetTrait<T::AssetId, T::AccountId, T::Balance, T::Cost, T::Income> for Module<T> {
	fn asset_create(symbol: Vec<u8>, precision: u16) -> (T::AssetId, TokenPair<T::Balance>) {
		let id = match symbol.as_slice() {
			b"DOT" => 0.into(),
			b"KSM" => 1.into(),
			b"EOS" => 2.into(),
			_ => {
				let id = Self::next_asset_id();
				<NextAssetId<T>>::mutate(|id| *id += One::one());
				id
			}
		};

		// Initial total supply is zero.
		let total_supply: T::Balance = 0.into();

		// Create token
		let token = Token::new(symbol.clone(), precision, total_supply);
		let vtoken = Token::new(symbol, precision, total_supply);
		let token_pair = TokenPair::new(token, vtoken);

		// Insert to storage
		<Tokens<T>>::insert(id, token_pair.clone());

		(id, token_pair)
	}

	fn asset_issue(
		asset_id: T::AssetId,
		token_type: TokenType,
		target: T::AccountId,
		amount: T::Balance,
	) {
		let convert_rate = T::FetchConvertPrice::fetch_convert_price(asset_id);
		let target_asset = (asset_id, token_type, target.clone());
		<AccountAssets<T>>::mutate(&target_asset, |asset| {
			asset.balance = asset.balance.saturating_add(amount);
			asset.cost = asset.cost.saturating_add(amount.saturating_mul(convert_rate.into()).into());
		});

		// save asset id for this account
		if <AccountAssetIds<T>>::contains_key(&target) {
			<AccountAssetIds<T>>::mutate(&target, |ids| {
				if !ids.contains(&asset_id) { // do not push a duplicated asset id to list
					ids.push(asset_id);
				}
			});
		} else {
			<AccountAssetIds<T>>::insert(&target, vec![asset_id]);
		}

		<Tokens<T>>::mutate(asset_id, |token| {
			match token_type {
				TokenType::Token => {
					token.token.total_supply = token.token.total_supply.saturating_add(amount);
				},
				TokenType::VToken => {
					token.vtoken.total_supply = token.vtoken.total_supply.saturating_add(amount);
				}
			}
		});
	}

	fn asset_redeem(
		asset_id: T::AssetId,
		token_type: TokenType,
		target: T::AccountId,
		amount: T::Balance,
	) {
		Self::asset_destroy(asset_id, token_type, target, amount);
	}

	fn asset_destroy(
		asset_id: T::AssetId,
		token_type: TokenType,
		target: T::AccountId,
		amount: T::Balance,
	) {
		let convert_rate = T::FetchConvertPrice::fetch_convert_price(asset_id);
		let target_asset = (asset_id, token_type, target);
		<AccountAssets<T>>::mutate(&target_asset, |asset| {
			asset.balance = asset.balance.saturating_sub(amount);
			asset.income = asset.income.saturating_add(amount.saturating_mul(convert_rate.into()).into());
		});

		<Tokens<T>>::mutate(asset_id, |token| {
			match token_type {
				TokenType::Token => {
					token.token.total_supply = token.token.total_supply.saturating_sub(amount);
				},
				TokenType::VToken => {
					token.vtoken.total_supply = token.vtoken.total_supply.saturating_sub(amount);
				}
			}
		});
	}

	fn asset_id_exists(who: &T::AccountId, symbol: &[u8], precision: u16) -> Option<T::AssetId> {
		let all_ids = <AccountAssetIds<T>>::get(who);
		for id in all_ids {
			let token = <Tokens<T>>::get(id);
			if token.token.symbol.as_slice().eq(symbol) && token.token.precision.eq(&precision) {
				return Some(id);
			}
		}
		None
	}

	fn token_exists(asset_id: T::AssetId) -> bool {
		<Tokens<T>>::contains_key(&asset_id)
	}

	fn get_account_asset(
		asset_id: &T::AssetId,
		token_type: TokenType,
		target: &T::AccountId,
	) -> AccountAsset<T::Balance, T::Cost, T::Income> {
		<AccountAssets<T>>::get((&asset_id, token_type, &target))
	}

	fn get_token(asset_id: &T::AssetId) -> TokenPair<T::Balance> {
		<Tokens<T>>::get(&asset_id)
	}
}

impl<T: Trait> TokenPriceHandler<T::Price> for Module<T> {
	fn set_token_price(symbol: Vec<u8>, price: T::Price) {
		match TOKEN_LIST.iter().position(|s| *s == symbol) {
			Some(id) => <Prices<T>>::mutate((T::AssetId::from(id as u32), TokenType::Token), |p| *p = price),
			_ => {},
		}
	}
}

// The main implementation block for the module.
impl<T: Trait> Module<T> {
	fn asset_transfer(
		asset_id: T::AssetId,
		token_type: TokenType,
		from: T::AccountId,
		to: T::AccountId,
		amount: T::Balance,
	) {
		let from_asset = (asset_id, token_type, from);
		<AccountAssets<T>>::mutate(&from_asset, |asset| {
			asset.balance = asset.balance.saturating_sub(amount);
		});

		let to_asset = (asset_id, token_type, &to);
		<AccountAssets<T>>::mutate(to_asset, |asset| {
			asset.balance = asset.balance.saturating_add(amount);
		});

		// save asset id for this account
		if <AccountAssetIds<T>>::contains_key(&to) {
			<AccountAssetIds<T>>::mutate(&to, |ids| {
				// do not push a duplicated asset id to list
				if !ids.contains(&asset_id) { ids.push(asset_id); }
			});
		} else {
			<AccountAssetIds<T>>::insert(&to, vec![asset_id]);
		}
	}

	pub fn asset_balances(asset_id: T::AssetId, token_type: TokenType, target: T::AccountId) -> u64 {
		let origin_account = (asset_id, token_type, target);
		let balance_u128 = <AccountAssets<T>>::get(origin_account).balance;

		// balance type is u128, but serde cannot serialize u128.
		// So I have to convert to u64, see this link
		// https://github.com/paritytech/substrate/issues/4641
		let balance_u64: u64 = balance_u128.try_into().unwrap_or(usize::max_value()) as u64;

		balance_u64
	}

	pub fn asset_tokens(target: T::AccountId) -> Vec<T::AssetId> {
		<AccountAssetIds<T>>::get(target)
	}
}
