#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;
use mangata_types::assets::CustomMetadata;

benchmarks! {
		where_clause { where T::CustomMetadata: From<CustomMetadata>, T::AssetId: From<u32>, T::Balance: From<u32> }

		register_asset {
			let metadata = AssetMetadata::<<T as module::Config>::Balance, <T as module::Config>::CustomMetadata> {
				decimals: 12,
				name: b"token".to_vec(),
				symbol: b"TOK".to_vec(),
				additional: CustomMetadata::default().into(),
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
			let name = b"new".to_vec();
			let symbol = b"NW".to_vec();
			let ex_deposit: T::Balance = 18.into();
			let location = Some(MultiLocation::new(1,X1(Parachain(3000))).into());
			let additional = CustomMetadata::default().into();
		}: _(RawOrigin::Root,
			asset_id,
			Some(decimals),
			Some(name),
			Some(symbol),
			Some(ex_deposit),
			Some(location),
			Some(additional))
}
