pub fn init_logger() {
    let base_config = fern::Dispatch::new();

    let console_config =
        fern::Dispatch::new()
            .level(log::LevelFilter::Trace)
            .format(|out, message, record| {
                out.finish(format_args! {
                    "[{}] {}:{} {} {}",
                    record.level(),
                    record.file().unwrap(),
                    record.line().unwrap(),
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    message
                })
            });

    let application_config = fern::Dispatch::new()
        .level(log::LevelFilter::Info)
        .format(|out, message, record| {
            out.finish(format_args! {
                "[{}] {}:{} {} {}",
                record.level(),
                record.file().unwrap(),
                record.line().unwrap(),
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                message
            })
        })
        .chain(std::io::stdout());
    // .chain(fern::log_file("application.log").unwrap());

    let emergency_config = fern::Dispatch::new()
        .level(log::LevelFilter::Error)
        .format(|out, message, record| {
            out.finish(format_args! {
                "[{}] {}:{} {} {}",
                record.level(),
                record.file().unwrap(),
                record.line().unwrap(),
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                message
            })
        })
        .chain(fern::log_file("emergency.log").unwrap());

    base_config
        .chain(console_config)
        .chain(application_config)
        .chain(emergency_config)
        .apply()
        .unwrap();
}
