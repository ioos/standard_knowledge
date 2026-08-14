use standard_knowledge::standards_library::StandardsLibrary;

fn main() {
    let mut library = StandardsLibrary::default();
    #[cfg(feature = "embedded-data")]
    library.load_cf_standards();
    println!(
        "By name: {:?}",
        &library.get("air_pressure_at_mean_sea_level")
    );
    println!("By alias: {:?}", &library.get("air_pressure_at_sea_level"));
}
