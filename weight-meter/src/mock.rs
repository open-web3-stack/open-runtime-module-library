#[frame_support::pallet]
pub mod test_module {
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, weights::Weight};
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::storage]
	#[pallet::getter(fn something)]
	pub type Something<T> = StorageValue<_, u32>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(50_000)]
		#[orml_weight_meter::start]
		pub fn expect_100(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			Self::put_100();

			Ok(Some(orml_weight_meter::used_weight()).into())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(50_000)]
		#[orml_weight_meter::start]
		pub fn expect_500(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			Self::put_100();
			Self::put_100();
			Self::put_100();
			Self::put_100();
			Self::put_100();

			Ok(Some(orml_weight_meter::used_weight()).into())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(50_000)]
		#[orml_weight_meter::start]
		pub fn expect_max_weight(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			Self::max_weight();
			Self::put_100();

			Ok(Some(orml_weight_meter::used_weight()).into())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(50_000)]
		#[orml_weight_meter::start]
		pub fn expect_100_or_200(origin: OriginFor<T>, branch: bool) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			if branch {
				Self::put_200();
			} else {
				Self::put_100();
			}

			Ok(Some(orml_weight_meter::used_weight()).into())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(50_000)]
		#[orml_weight_meter::start]
		pub fn nested_inner_methods(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			Self::put_300_nested();

			Ok(Some(orml_weight_meter::used_weight()).into())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(50_000)]
		#[orml_weight_meter::start]
		pub fn nested_extrinsic(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			ensure_signed(origin.clone())?;

			// some module call
			Self::put_300_nested();

			// call extrinsic method
			Self::expect_100(origin)?;

			// some module call
			Self::put_300_nested();

			Ok(Some(orml_weight_meter::used_weight()).into())
		}
	}

	impl<T: Config> Pallet<T> {
		#[orml_weight_meter::weight(100)]
		fn put_100() {
			let something = Self::something();

			if let Some(v) = something {
				Something::<T>::put(v.checked_add(100).unwrap());
			} else {
				Something::<T>::put(100);
			}
		}

		#[orml_weight_meter::weight(200)]
		fn put_200() {
			let something = Self::something();

			if let Some(v) = something {
				Something::<T>::put(v.checked_add(200).unwrap());
			} else {
				Something::<T>::put(100);
			}
		}

		#[orml_weight_meter::weight(200)]
		fn put_300_nested() {
			Self::put_100();
		}

		#[orml_weight_meter::weight(Weight::MAX.ref_time())]
		fn max_weight() {}
	}
}

use frame_support::sp_runtime::traits::IdentityLookup;
use frame_support::traits::{ConstU128, ConstU32, ConstU64, Everything};
use sp_runtime::testing::{Header, H256};

pub type BlockNumber = u64;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;
type Balance = u128;

impl frame_system::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = frame_system::Pallet<Runtime>;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

impl test_module::Config for Runtime {}

frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		TestModule: test_module::{Pallet, Call, Storage},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

pub struct ExtBuilder();

impl Default for ExtBuilder {
	fn default() -> Self {
		Self()
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: vec![(100, 100_000)],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	ExtBuilder::default().build()
}
