use pretty_assertions::assert_eq;

#[orml_weight_meter::weight(100)]
fn foo() {
    // do something
}

#[orml_weight_meter::weight(250)]
fn bar() {
    // do something
}

pub fn main() {
    assert_eq!(orml_weight_meter::used_weight(), 0);
    foo();
    assert_eq!(orml_weight_meter::used_weight(), 100);
    bar();
    assert_eq!(orml_weight_meter::used_weight(), 350);
}
