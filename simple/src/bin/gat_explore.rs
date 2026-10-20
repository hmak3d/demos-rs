//! How to capture lifetimes for return type of higher kinded functions.
//!
//! RPIT in context of lower kind can use `+ use<>` but RPIT cannot.
//! But for higher kinded, you cannot use that.
//! Instead a Generic Associated Type (GAT) within a trait needs be returned.

#[derive(Default)]
struct Entry {
    name: String,
    country: String,
    age: usize,
}

////////////////////////////////////////////////////////////////////////////////
// Attempt #1: Broken naive impl (only works with &str)

fn get_width_str_ref<'a>(
    entries: impl IntoIterator<Item = &'a Entry>,
    get_field: impl Fn(&Entry) -> &str,
) -> usize {
    entries
        .into_iter()
        .map(|entry| get_field(entry).len())
        .min()
        .unwrap_or_default()
}

////////////////////////////////////////////////////////////////////////////////
// Attempt #2: Broken generic attempt (only works for *owned* impl of AsRef)

fn get_width_owned_only<'a, F, S>(
    entries: impl IntoIterator<Item = &'a Entry>,
    get_field: F,
) -> usize
where
    F: Fn(&Entry) -> S,
    S: AsRef<str>, // This cannot _not_ capture lifetine &Entry of F => will force get_field() to only return owned/static types, not borrowed types
{
    entries
        .into_iter()
        .map(|entry| get_field(entry).as_ref().len())
        .min()
        .unwrap_or_default()
}

////////////////////////////////////////////////////////////////////////////////
// Attempt #3: Ok GAT impl (more general)

trait FieldFamily {
    type Field<'b>: AsRef<str>;
}

struct StringFamily;
struct StringRefFamily;
struct StrRefFamily;

impl FieldFamily for StringFamily {
    type Field<'b> = String;
}

impl FieldFamily for StringRefFamily {
    type Field<'b> = &'b String;
}

impl FieldFamily for StrRefFamily {
    type Field<'b> = &'b str;
}

fn get_width<'a, FF: FieldFamily>(
    entries: impl IntoIterator<Item = &'a Entry>,
    get_field: impl for<'b> Fn(&'b Entry) -> FF::Field<'b>,
) -> usize {
    entries
        .into_iter()
        .map(|entry| get_field(entry).as_ref().len())
        .min()
        .unwrap_or_default()
}

////////////////////////////////////////////////////////////////////////////////
// Exercise implementations

#[allow(unused_variables)]
fn main() {
    let entries = [Entry::default(), Entry::default()];

    // Attempt #1: Broken naive impl (only works with &str)

    let name_width = get_width_str_ref(&entries, |entry| &entry.name); // ok; because closure returns &String which can deref coerce into &str
    // let country_width = get_width_str_ref(&entries, |entry| entry.country); // BAD; does not compile because closure returns String not &str

    // Attempt #2: Broken generic attempt (only works for *owned* impl of AsRef)

    // let name_width = get_width_owned_only(&entries, |entry| /* -> &String */ &entry.name); // BAD; does not compile because closure must return owned/static types
    // let country_width = get_width_owned_only(&entries, |entry| /* -> &str */ &*entry.country); // BAD; does not compile because closure must return owned/static types
    let age_width = get_width_owned_only(&entries, |entry| /* -> String */ entry.age.to_string()); // ok; because closure returns owned/static type

    // Attempt #3: Ok GAT impl (more general)

    let name_width =
        get_width::<'_, StringRefFamily>(&entries, |entry| /* -> &String */ &entry.name); // ok; closure returns &String
    let country_width =
        get_width::<'_, StrRefFamily>(&entries, |entry| /* -> &str */ &*entry.country); // ok; closure returns &str
    let age_width =
        get_width::<'_, StringFamily>(&entries, |entry| /* -> String */ entry.age.to_string()); // ok; closure returns String
}
