#[orml_weight_meter::weight(100)]
fn foo() {
    // do something
}

#[orml_weight_meter::weight(250)]
fn bar() {
    // do something
}

pub fn main() {
    assert!(orml_weight_meter::used_weight() == 0);
    foo();
    assert!(orml_weight_meter::used_weight() == 100);
    bar();
    assert!(orml_weight_meter::used_weight() == 350);
}
