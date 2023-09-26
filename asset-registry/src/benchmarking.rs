#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;

benchmarks! {
		where_clause { where T::CustomMetadata: Default, T::AssetId: From<u32>, T::Balance: From<u32> }

		register_asset {
			let metadata = AssetMetadata::<<T as module::Config>::Balance, <T as module::Config>::CustomMetadata, <T as module::Config>::StringLimit> {
				decimals: 12,
				name: BoundedVec::truncate_from("token".as_bytes().to_vec()),
				symbol: BoundedVec::truncate_from("TOK".as_bytes().to_vec()),
				additional: <T as module::Config>::CustomMetadata::default(),
				existential_deposit: 0.into(),
				location: Some(
					MultiLocation::new(
						1,
						X1(Parachain(2000)),
					)
					.into(),
				),
			};
		}: _(RawOrigin::Root, metadata, None)

		update_asset {
			let asset_id: T::AssetId = 1.into();
			let decimals = 18;
			let name = BoundedVec::truncate_from("new token".as_bytes().to_vec());
			let symbol = BoundedVec::truncate_from("NWT".as_bytes().to_vec());
			let ex_deposit: T::Balance = 18.into();
			let location = Some(MultiLocation::new(1,X1(Parachain(3000))).into());
			let additional = <T as module::Config>::CustomMetadata::default();
		}: _(RawOrigin::Root,
			asset_id,
			Some(decimals),
			Some(name),
			Some(symbol),
			Some(ex_deposit),
			Some(location),
			Some(additional))
}
