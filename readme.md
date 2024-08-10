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

用 Rust 编写的 mongo 压力测试工具。

## Feature

- 压力测试
- 压力回放
- 逐步增压
- TUI 操作
- 高性能高响应
- 统计报告生成
- 日志录制
- 实时数据统计
- tokio 的无限线程

## Install

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh # install cargo

cargo install mongobar
```

## Use

```
mongobar --help
```

> 特别提示：
> 在进行 写 压力测试的时候请注意数据库的备份。

### 1.我们先创建一个配置文件

> mongobar.json

```json
{
  "uri": "mongodb://root:12345678@localhost:27017/?authSource=admin",
  "db": "qxg",
  "thread_count": 1000,
  "loop_count": 100
}
```

上面比较简单我就不多说了。

### 2.录制一段操作的日志

```
mongobar op-record oplogsname -f

# oplogsname 最后要压力测试或者重放的名称
# -f 如果之前录制过则覆盖
```

原理是开启 profileLevel 2 进行全量日志采集，采集完成之后输入 `Y` 就可以生成了 oplogs.op 文件，当执行命令的时候就算是开始录制，Y 之后就是结束采集，结束采集后会将 profileLevel 设置成 0 或者 1（恢复设置）。

这里的 \*.op 文件是最终要跑的测试脚本文件。

生成的 .op 文件会放在 `.mongobar/oplogsname/oplogs.op`.

### 3.将上面录制的日志进行压力测试

有两种方式可以执行压力测试：

- 直接通过命令
- 通过 TUI 操作

#### 直接通过命令

```
mongobar op-stress oplogsname -l 10 -t 128

# op-stress 是这个命令
# oplogsname 是压测的例子
# -l 是循环次数， 0 表示无限次数
# -t 表示线程数，也就是并发
```

#### 通过 TUI 操作

```
mongobar ui oplogsname -l 10 -t 128

# op-stress 是这个命令
# oplogsname 是压测的例子
# -l 是循环次数， 0 表示无限次数
# -t 表示线程数，也就是并发
```

执行上面的命令会打开一个 tui 界面，我们可以通过 UP 、Down、Enter 来操作。

我们可以通过这个路径进入进行压力测试 UI -> [Stress] -> [Start].

![alt text](./assets/image.png)

- Boost+
  - 我们可以通过这个选项动态添加线程数，这个只能加
- CCLimit
  - 我们可以现在当前并发多少
- Stop
  - 停止并生成 `query_stats.csv` 测试报告
- Back
  - 停止并退出

### 4.回放日志文件

日志回放与上面区别就是：

- 只会执行一遍
- 拿的时候有一定顺序执行

注意：

- 在压力测试前请先执行 UI -> [Reply] -> [Revert] 生成恢复数据和重置到测试前的状态（具体逻辑可以看 src/mongobar/mod.rs）。
- 再次提示请做好数据库备份。

我们可以通过一些方法来采集到一些完整的数据执行命令的片段，比如我就是从阿里云的审计日志拉取的 100w 条日志进行日志回放的压力测试的，为此我还开发一些命令用来将阿里审计日志的 csv 转为 mongobar 的 op 文件 `mongobar tool cov xxx.csv`。

我们可以操作：

UI -> [Replay] -> [Revert]

完成之后执行:

UI -> [Replay] -> [Start]

测试完成后执行：

UI -> [Replay] -> [Resume]

进行恢复。

> 提示：
>
> - 我在实践过程中其实 replay 功能还是无法对大数据压崩，但是工具也不支持分布式集群压测，所以该功能还是有很多局限的，不过我操作的方式是开多个 linux，然后使用 termius 的广播功能统一处理。

## About

@班级小管家/lyda
