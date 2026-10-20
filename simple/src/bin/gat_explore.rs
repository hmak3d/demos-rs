//! How to capture lifetimes for return type of higher kinded functions.
//!
//! A function that is RPIT can `+ use<>` for good ergonomics.
//! However, if that function itself uses RPIT for its return value, this doesn't work.
//! Instead, a Generic Associated Type (GAT) within a trait needs be returned.
//!
//! See [GAT RFC 1598](https://rust-lang.github.io/rfcs/1598-generic_associated_types.html)

#[derive(Default)]
struct Entry {
    name: String,
    country: String,
    age: usize,
}

////////////////////////////////////////////////////////////////////////////////
// Attempt #1: Broken naive impl (only works with projections to &str)

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
// Attempt #2: Broken generic attempt (only works for projections that are *owned* impl of AsRef)

fn get_width_owned_only<'a, F, S>(
    entries: impl IntoIterator<Item = &'a Entry>,
    get_field: F,
) -> usize
where
    F: Fn(&Entry) -> S,
    // This cannot _not_ capture lifetine &Entry parameter of of F
    // => will force get_field() to only return owned/static types, never borrowed types
    S: AsRef<str>,
    // The following is _not_ possible
    // S: AsRef<str> + use<lifetime of &Entry>
{
    entries
        .into_iter()
        .map(|entry| get_field(entry).as_ref().len())
        .min()
        .unwrap_or_default()
}

////////////////////////////////////////////////////////////////////////////////
// Attempt #3: Ok GAT impl (more general; works for projections to any AsRef<str> regardless of owned (e.g., String) or
// borrowed (e.g., &String, &str)

/// This trait has two associated types that look similar but are completely different
///
/// |                                   | [FieldFamily::UselessLowerKind]   | [FieldFamily::Field]              |
/// | ---                               | ---                               | ---                               |
/// | kind | type | lifetime -&gt; type |                                   |                                   |
/// | to get concrete type, caller ...  | can use directly                  | must supply a lifetime            |
/// | notation                          | no &lt;...&gt; to left of `:`     | has &lt;...&gt; to left of `:`    |
/// | dyn compatibility                 | normally no problem               | makes [FieldFamily] _not_ dyn compatible |
trait FieldFamily<'f> {
    /// This is a of kind "lifetime -> type" (a higher kind)
    /// `Field` captures `'c` ... so it lives at most `'c`.
    /// This is the opposite of [FieldFamily::AnotherUselessLowerKind].
    type Field<'c>: AsRef<str>;

    /// This is a of kind "type" (a lower kind)
    type UselessLowerKind: AsRef<str>;

    /// `AnotherUselessLowerKind` lives at least `'f`.
    /// This is the opposite of [FieldFamily::Field].
    type AnotherUselessLowerKind: AsRef<str> + 'f;
}

struct StringFamily;
struct StringRefFamily;
struct StrRefFamily;

impl<'f> FieldFamily<'f> for StringFamily {
    type Field<'c> = String;
    type UselessLowerKind = String;
    type AnotherUselessLowerKind = String;
}

impl<'f> FieldFamily<'f> for StringRefFamily {
    type Field<'c> = &'c String;
    type UselessLowerKind = String;
    type AnotherUselessLowerKind = String;
}

impl<'f> FieldFamily<'f> for StrRefFamily {
    type Field<'c> = &'c str;
    type UselessLowerKind = String;
    type AnotherUselessLowerKind = String;
}

fn get_width<'a, 'b, FF: FieldFamily<'b>>(
    entries: impl IntoIterator<Item = &'a Entry>,
    get_field: impl for<'x> Fn(&'x Entry) -> FF::Field<'x>,
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
        get_width::<'_, '_, StringRefFamily>(&entries, |entry| /* -> &String */ &entry.name); // ok; closure returns &String
    let country_width =
        get_width::<'_, '_, StrRefFamily>(&entries, |entry| /* -> &str */ &*entry.country); // ok; closure returns &str
    let age_width =
        get_width::<'_, '_, StringFamily>(&entries, |entry| /* -> String */ entry.age.to_string()); // ok; closure returns String
}
