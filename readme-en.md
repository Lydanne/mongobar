<div align="center">
  <p><img src="logo.png" width="50%" /></p>
  <p>
    <img src="https://img.shields.io/crates/v/mongobar?style=for-the-badge" />
    <img src="https://img.shields.io/github/license/WumaCoder/mongobar?style=for-the-badge" />
  </p>
  <p>
    <a href="https://github.com/WumaCoder/mongobar/blob/master/readme-en.md">English</a>
  </p>
</div>

# Mongobar

<p align="center">
    <img src="./assets/home.png" width="45%" />
    <img src="./assets/stress.png" width="45%" />
</p>

A mongo stress testing tool written in Rust.

## Feature

- Stress testing
- Stress playback
- Gradual pressure increase
- TUI operation
- High performance and high response
- Generation of statistical reports
- Log recording
- Real-time data statistics
- Unlimited threads of tokio

## Install

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh # install cargo

cargo install mongobar
```

## Use

```
mongobar --help
```

> Special note:
> Please pay attention to the backup of the database when conducting write stress tests.

### 1. Let's first create a configuration file

> mongobar.json

```json
{
  "uri": "mongodb://root:12345678@localhost:27017/?authSource=admin",
  "db": "qxg",
  "thread_count": 1000,
  "loop_count": 100
}
```

The above is relatively simple, so I won't elaborate.

### 2. Record a log of an operation

```
mongobar op-record oplogsname -f

# oplogsname is the name of the final stress test or replay
# -f If it has been recorded before, overwrite
```

The principle is to enable profileLevel 2 for full log collection. After the collection is completed, enter `Y` to generate the oplogs.op file. When the command is executed, it is considered the start of recording. After `Y`, it is the end of collection. After the collection is ended, profileLevel will be set to 0 or 1 (restore the setting).

The \*.op file here is the final test script file to be run.

The generated.op file will be placed in `.mongobar/oplogsname/oplogs.op`.

### 3. Conduct stress tests on the recorded logs above

There are two ways to perform stress tests:

- Directly through commands
- Through TUI operation

#### Directly through commands

```
mongobar op-stress oplogsname -l 10 -t 128

# op-stress is this command
# oplogsname is the stress test example
# -l is the number of loops, 0 means unlimited times
# -t represents the number of threads, that is, concurrency
```

#### Through TUI operation

```
mongobar ui oplogsname -l 10 -t 128

# op-stress is this command
# oplogsname is the stress test example
# -l is the number of loops, 0 means unlimited times
# -t represents the number of threads, that is, concurrency
```

Executing the above command will open a tui interface, and we can operate through UP, Down, and Enter.

We can enter for stress testing through this path UI -> [Stress] -> [Start].

![alt text](./assets/image.png)

- Boost+
  - We can dynamically add the number of threads through this option, and this can only be added
- CCLimit
  - We can now know how much the current concurrency is
- Stop
  - Stop and generate the `query_stats.csv` test report
- Back
  - Stop and exit

### 4. Replay the log file

The difference between log replay and the above is:

- It will only be executed once
- It is executed in a certain order when fetched

Note:

- Before the stress test, please first execute UI -> [Reply] -> [Revert] to generate restored data and reset to the state before the test (for specific logic, you can see src/mongobar/mod.rs).
- Again, please make a database backup.

We can collect some complete data execution command fragments through some methods. For example, I conducted a stress test of log replay by pulling 1 million logs from the audit logs of Alibaba Cloud. For this, I also developed some commands to convert the csv of Alibaba Cloud's audit logs to the op file of mongobar `mongobar tool cov xxx.csv`.

We can operate:

UI -> [Replay] -> [Revert]

After completion, execute:

UI -> [Replay] -> [Start]

After the test is completed, execute:

UI -> [Replay] -> [Resume]

For recovery.

> Tips:
>
> - In my practical process, in fact, the replay function still cannot crush under large data pressure, and the tool does not support distributed cluster stress testing, so this function still has many limitations. However, my operation method is to open multiple linux and then use the broadcast function of termius for unified processing.

## About

@xgj/lyda
