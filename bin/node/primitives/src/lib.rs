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

//! Low-level types used throughout the Bifrost code.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use sp_runtime::{
	generic, traits::{Verify, BlakeTwo256, IdentifyAccount}, OpaqueExtrinsic, MultiSignature
};
use sp_std::prelude::*;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them.
pub type AccountIndex = u32;

/// An index to an asset
pub type AssetId = u32;

/// Convert type
pub type ConvertPrice = u128;
pub type RatePerBlock = u64;

/// Balance of an account.
pub type Balance = u128;

/// Cost of an asset of an account.
pub type Cost = u128;

/// Income of an asset of an account.
pub type Income = u128;

/// Price of an asset.
pub type Price = u64;

/// Precision of symbol.
pub type Precision = u32;

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// A timestamp: milliseconds since the unix epoch.
/// `u64` is enough to represent a duration of half a billion years, when the
/// time scale is milliseconds.
pub type Timestamp = u64;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;
/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type.
pub type Block = generic::Block<Header, OpaqueExtrinsic>;
/// Block ID.
pub type BlockId = generic::BlockId<Block>;

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum TokenType {
	Token,
	VToken,
}

impl Default for TokenType {
	fn default() -> Self {
		Self::VToken
	}
}

#[derive(Debug, Decode, Encode, Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum AssetSymbol {
	DOT = 0,
	KSM,
	EOS,
}

impl From<AssetId> for AssetSymbol {
	fn from(id: AssetId) -> Self {
		match id {
			0 => Self::DOT,
			1 => Self::KSM,
			2 => Self::EOS,
			_ => unimplemented!("This asset id is not sopported now.")
		}
	}
}

impl From<AssetSymbol> for AssetId {
	fn from(symbol: AssetSymbol) -> Self {
		match symbol {
			AssetSymbol::DOT => 0,
			AssetSymbol::KSM => 1,
			AssetSymbol::EOS => 2,
		}
	}
}

/// Token pair to bond token and vtoken
#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct TokenPair<Balance> {
	pub token: Token<Balance>,
	pub vtoken: Token<Balance>,
}

impl<Balance> TokenPair<Balance> {
	pub fn new(token: Token<Balance>, vtoken: Token<Balance>) -> Self {
		Self {
			token,
			vtoken,
		}
	}
}

/// Token type
#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Token<Balance> {
	pub symbol: Vec<u8>,
	pub precision: u16,
	pub total_supply: Balance,
}

impl<Balance> Token<Balance> {
	pub fn new(symbol: Vec<u8>, precision: u16, total_supply: Balance) -> Self {
		Self {
			symbol,
			precision,
			total_supply,
		}
	}
}

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountAsset<Balance, Cost, Income> {
	pub balance: Balance,
	pub cost: Cost,
	pub income: Income,
}

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct ConvertPool<Balance> {
	/// A pool that hold the total amount of token converted to vtoken
	pub token_pool: Balance,
	/// A pool that hold the total amount of vtoken converted from token
	pub vtoken_pool: Balance,
	/// Total reward for current convert duration
	pub current_reward: Balance,
	/// Total reward for next convert duration
	pub pending_reward: Balance,
}

impl<Balance: Default + Copy> ConvertPool<Balance> {
	pub fn new(token_amount: Balance, vtoken_amount: Balance) -> Self {
		Self {
			token_pool: token_amount,
			vtoken_pool: vtoken_amount,
			..Default::default()
		}
	}

	pub fn new_round(&mut self) {
		self.current_reward = self.pending_reward;
		self.pending_reward = Default::default();
	}
}

/// Clearing handler for assets change
pub trait ClearingHandler<AssetId, AccountId, BlockNumber, Balance> {
	/// Clearing for assets change
	fn asset_clearing(
		asset_id: AssetId,
		target: AccountId,
		last_block: BlockNumber,
		prev_amount: Balance,
		curr_amount: Balance,
	);

	/// Clearing for token change
	fn token_clearing(
		asset_id: AssetId,
		last_block: BlockNumber,
		prev_amount: Balance,
		curr_amount: Balance,
	);
}

impl<A, AC, BN, B> ClearingHandler<A, AC, BN, B> for () {
	fn asset_clearing(_: A, _: AC, _: BN, _: B, _: B) {}
	fn token_clearing(_: A, _: BN, _: B, _: B) {}
}

pub trait AssetTrait<AssetId, AccountId, Balance, Cost, Income> {
	fn asset_create(symbol: Vec<u8>, precision: u16) -> (AssetId, TokenPair<Balance>);

	fn asset_issue(asset_id: AssetId, token_type: TokenType, target: AccountId, amount: Balance);

	fn asset_redeem(asset_id: AssetId, token_type: TokenType, target: AccountId, amount: Balance);

	fn asset_destroy(asset_id: AssetId, token_type: TokenType, target: AccountId, amount: Balance);

	fn asset_id_exists(who: &AccountId, symbol: &[u8], precision: u16) -> Option<AssetId>;

	fn token_exists(asset_id: AssetId) -> bool;

	fn get_account_asset(asset_id: &AssetId, token_type: TokenType, target: &AccountId) -> AccountAsset<Balance, Cost, Income>;

	fn get_token(asset_id: &AssetId) -> TokenPair<Balance>;
}

impl<AssetId, AccountId, Balance, Cost, Income> AssetTrait<AssetId, AccountId, Balance, Cost, Income> for ()
	where AssetId: Default, AccountId: Default, Balance: Default, Cost: Default, Income: Default
{
	fn asset_create(_: Vec<u8>, _: u16) -> (AssetId, TokenPair<Balance>) { Default::default() }

	fn asset_issue(_: AssetId, _: TokenType, _: AccountId, _: Balance) {}

	fn asset_redeem(_: AssetId, _: TokenType, _: AccountId, _: Balance) {}

	fn asset_destroy(_: AssetId, _: TokenType, _: AccountId, _: Balance) {}

	fn asset_id_exists(_: &AccountId, _: &[u8], _: u16) -> Option<AssetId> { Default::default() }

	fn token_exists(_: AssetId) -> bool { Default::default() }

	fn get_account_asset(_: &AssetId, _: TokenType, _: &AccountId) -> AccountAsset<Balance, Cost , Income> { Default::default() }

	fn get_token(_: &AssetId) -> TokenPair<Balance> { Default::default() }
}

pub trait TokenPriceHandler<Price> {
	fn set_token_price(symbol: Vec<u8>, price: Price);
}

impl<Price> TokenPriceHandler<Price> for () {
	fn set_token_price(_: Vec<u8>, _: Price) {}
}

/// Asset redeem handler
pub trait AssetRedeem<AssetId, AccountId, Balance> {
	/// Asset redeem
	fn asset_redeem(asset_id: AssetId, token_type: TokenType, target: AccountId, amount: Balance, to_name: Option<Vec<u8>>);
}

impl<A, AC, B> AssetRedeem<A, AC, B> for () {
	fn asset_redeem(_: A, _: TokenType, _: AC, _: B, _: Option<Vec<u8>>) {}
}

/// Fetch convert rate handler
pub trait FetchConvertPrice<AssetId, ConvertPrice> {
	/// fetch convert rate
	fn fetch_convert_price(asset_id: AssetId) -> ConvertPrice;
}

impl<A, ER: Default> FetchConvertPrice<A, ER> for () {
	fn fetch_convert_price(_: A) -> ER { Default::default() }
}

/// Blockchain types
#[derive(PartialEq, Debug, Clone, Encode, Decode)]
pub enum BlockchainType {
	BIFROST,
	EOS,
}

impl Default for BlockchainType {
	fn default() -> Self {
		BlockchainType::BIFROST
	}
}

/// Symbol type of bridge asset
#[derive(Clone, Default, Encode, Decode)]
pub struct BridgeAssetSymbol<Precision> {
	pub blockchain: BlockchainType,
	pub symbol: Vec<u8>,
	pub precision: Precision,
}

impl<Precision> BridgeAssetSymbol<Precision> {
	pub fn new(blockchain: BlockchainType, symbol: Vec<u8>, precision: Precision) -> Self {
		BridgeAssetSymbol {
			blockchain,
			symbol,
			precision,
		}
	}
}

/// Bridge asset type
#[derive(Clone, Default, Encode, Decode)]
pub struct BridgeAssetBalance<AccountId, Precision, Balance> {
	pub symbol: BridgeAssetSymbol<Precision>,
	pub amount: Balance,
	pub memo: Vec<u8>,
	// store the account who send transaction to EOS
	pub from: AccountId,
	// which token type is sent to EOS
	pub token_type: TokenType,
}

/// Bridge asset from other blockchain to Bifrost
pub trait BridgeAssetFrom<AccountId, Precision, Balance> {
	fn bridge_asset_from(target: AccountId, bridge_asset: BridgeAssetBalance<AccountId, Precision, Balance>);
}

impl<A, P, B> BridgeAssetFrom<A, P, B> for () {
	fn bridge_asset_from(_: A, _: BridgeAssetBalance<A, P, B>) {}
}

/// Bridge asset from Bifrost to other blockchain
pub trait BridgeAssetTo<AccountId, Precision, Balance> {
	type Error;
	fn bridge_asset_to(target: Vec<u8>, bridge_asset: BridgeAssetBalance<AccountId, Precision, Balance>, ) -> Result<(), Self::Error>;
}

impl<A, P, B> BridgeAssetTo<A, P, B> for () {
	type Error = core::convert::Infallible;
	fn bridge_asset_to(_: Vec<u8>, _: BridgeAssetBalance<A, P, B>) -> Result<(), Self::Error> { Ok(()) }
}

pub trait AssetReward<AssetId, Balance> {
	fn set_asset_reward(token_id: AssetId, reward: Balance) -> Result<(), ()>;
}

impl<A, B> AssetReward<A, B> for () {
	fn set_asset_reward(_: A, _: B) -> Result<(), ()> { Ok(()) }
}


/// App-specific crypto used for reporting equivocation/misbehavior in BABE and
/// GRANDPA. Any rewards for misbehavior reporting will be paid out to this
/// account.
pub mod report {
	use super::{Signature, Verify};
	use frame_system::offchain::AppCrypto;
	use sp_core::crypto::KeyTypeId;

	/// Key type for the reporting module. Used for reporting BABE and GRANDPA
	/// equivocations.
	pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"fish");

	mod app {
		use sp_application_crypto::{app_crypto, sr25519};
		app_crypto!(sr25519, super::KEY_TYPE);
	}

	/// Identity of the equivocation/misbehavior reporter.
	pub type ReporterId = app::Public;

	/// An `AppCrypto` type to allow submitting signed transactions using the reporting
	/// application key as signer.
	pub struct ReporterAppCrypto;

	impl AppCrypto<<Signature as Verify>::Signer, Signature> for ReporterAppCrypto {
		type RuntimeAppPublic = ReporterId;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}
