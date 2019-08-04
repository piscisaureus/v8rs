use std::cell::Cell;
use std::marker::PhantomData;

// Dummy placeholders representing raw v8 objects.
struct V8Isolate {}
struct V8HandleScope {}

// Scope that controls access to the Isolate and active HandleScope.
struct Scope<'a, S, P> {
    parent: Option<&'a mut P>,
    v8_object: Cell<S>, // container for raw v8 Isolate or HandleScope.
}

impl<'a> Scope<'a, V8Isolate, ()> {
    fn new_isolate() -> Self {
        Scope {
            parent: None,
            v8_object: Cell::new(V8Isolate {}),
        }
    }
}

impl<'a, S, P> Scope<'a, S, P> {
    fn new_handle_scope<'n>(&'n mut self) -> Scope<'n, V8HandleScope, Self> {
        Scope {
            parent: Some(self),
            v8_object: Cell::new(V8HandleScope {}),
        }
    }

    fn drop(self) {}
}

struct Local<'sc> {
    val: i32,
    scope: PhantomData<&'sc V8HandleScope>,
}

impl<'sc> Local<'sc> {
    fn new<P>(_: &mut Scope<'sc, V8HandleScope, P>) -> Self {
        Self {
            val: 0,
            scope: PhantomData,
        }
    }

    fn alive(&self) {}
}

#[allow(unused_variables)]
fn main() {
    let local_in_scope3;

    let ref mut isolate = Scope::new_isolate();

    let ref mut scope1 = isolate.new_handle_scope();
    let local_a_in_scope1 = Local::new(scope1);
    let local_b_in_scope1 = Local::new(scope1);

    {
        let ref mut scope2 = scope1.new_handle_scope();
        let local_a_in_scope2 = Local::new(scope2);
        let local_b_in_scope2 = Local::new(scope2);

        // fail: scope1 is made inaccessible by scope2's existence.
        let mut _fail = scope1.new_handle_scope();
        // fail: same reason.
        let _fail = Local::new(scope1);

        {
            let mut scope3 = scope2.new_handle_scope();
            local_in_scope3 = Local::new(&mut scope3);

            let _fail = Local::new(scope1); // fail: scope1 locked by scope2
            let _fail = Local::new(scope2); // fail: scope2 locked by scope3

            // **BUG**: this is accepted but should not, because
            // local_in_scope3 is stil alive.
            scope3.drop();

            // fail: scope2 still locked because local_in_scope3 is alive,
            // so scope3 must be alive.
            let _fail = Local::new(scope2);

            local_in_scope3.alive();

            // pass: local_in_scope3 not used after this, so it can drop
            // => therefore, scope3 can drop
            // => therefore, scope2 can be used again.
            let local_c_in_scope2 = Local::new(scope2);
        }

        // fail: scope1 not accessible, because local_a_in_scope2 is keeping
        // scope2 alive.
        let _fail = Local::new(scope1);

        local_a_in_scope2.alive();

        // pass: local_a_in_scope2 can drop, scope1 accessible again.
        let local_c_in_scope1 = Local::new(scope1);
    }

    let local_c_in_scope1 = Local::new(scope1);
    local_a_in_scope1.alive();

    // Uncommenting this should make all scope1/scope2 uses after
    // local_in_scope3's creation fail.
    // local_in_scope3.alive();
}
