pub trait OnNewData<Key: Clone, Value: Clone> {
	fn on_new_data(key: &Key, value: &Value);
}

impl<Key: Clone, Value: Clone> OnNewData<Key, Value> for () {
	fn on_new_data(key: &Key, value: &Value) {}
}
