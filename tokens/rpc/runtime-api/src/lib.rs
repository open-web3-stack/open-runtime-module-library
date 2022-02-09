// This file is part of Substrate.

// Copyright (C) 2019-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Runtime API definition for transaction payment pallet.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::pallet_prelude::*;
use sp_runtime::traits::{MaybeDisplay, MaybeSerializeDeserialize, Member};

sp_api::decl_runtime_apis! {
	pub trait TokensApi<CurrencyId, Balance> where
		Balance: Codec + MaybeDisplay,
		CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord
	{
		fn query_existential_deposit(currency_id: CurrencyId) -> Balance;
	}
}
