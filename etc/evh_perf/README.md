# EVH Perf

EVH Perf is a tool that tests the performance of the eventhandler. The tool can be run in either `eventhandler` mode or `client` mode. Both modes may be run at the same time as well by specifying both the -c and the -e option. The --help option lists all available configuration options. The output of the tool running in eventhandler mode might look like this:

```
$ ./target/release/evh_perf -e --debug --host 0.0.0.0 --max_handles_per_thread 1000 --port 8082 --read_slab_count 500 --reuse_port --threads 10 --tls
[2024-02-08 10:27:00.583]: evh_perf EventHandler/0.0.3-beta.1
----------------------------------------------------------------------------------------------------
[2024-02-08 10:27:00.583]: debug:                  'true'
[2024-02-08 10:27:00.583]: host:                   '0.0.0.0'
[2024-02-08 10:27:00.583]: max_handles_per_thread: '1,000'
[2024-02-08 10:27:00.583]: port:                   '8082'
[2024-02-08 10:27:00.583]: read_slab_count:        '500'
[2024-02-08 10:27:00.583]: reuse_port:             'true'
[2024-02-08 10:27:00.583]: threads:                '10'
[2024-02-08 10:27:00.583]: tls:                    'true'
----------------------------------------------------------------------------------------------------
[2024-02-08 10:27:00.603]: (INFO) Server started in 26 ms.
```

To run the evh_perf tool in client mode with the coresponding eventhandler (above), the following options might be specified:

```
$ ./target/release/evh_perf -c --host 127.0.0.1 --max_handles_per_thread 100 --port 8082 --tls --read_slab_count 500 --threads 2 --itt 2 --count 2 --clients 2 --histo --min 20 --max 30 --reconns 2 --sleep 10
[2024-02-08 10:33:19.613]: evh_perf Client/0.0.3-beta.1
----------------------------------------------------------------------------------------------------
[2024-02-08 10:33:19.613]: clients:                '2'
[2024-02-08 10:33:19.613]: count:                  '2'
[2024-02-08 10:33:19.613]: debug:                  'false'
[2024-02-08 10:33:19.613]: histo:                  'true'
[2024-02-08 10:33:19.613]: histo_delta_micros:     '10'
[2024-02-08 10:33:19.613]: host:                   '127.0.0.1'
[2024-02-08 10:33:19.613]: iterations:             '2'
[2024-02-08 10:33:19.613]: max:                    '30'
[2024-02-08 10:33:19.613]: max_handles_per_thread: '100'
[2024-02-08 10:33:19.613]: min:                    '20'
[2024-02-08 10:33:19.613]: port:                   '8082'
[2024-02-08 10:33:19.613]: read_slab_count:        '500'
[2024-02-08 10:33:19.613]: reconns:                '2'
[2024-02-08 10:33:19.613]: sleep:                  '10'
[2024-02-08 10:33:19.613]: threads:                '2'
[2024-02-08 10:33:19.613]: tls:                    'true'
----------------------------------------------------------------------------------------------------
[2024-02-08 10:33:19.614]: (INFO) Client started in 7 ms.
[2024-02-08 10:33:19.663]: (INFO) sleeping for 10 ms.
[2024-02-08 10:33:19.663]: (INFO) sleeping for 10 ms.
[2024-02-08 10:33:19.723]: (INFO) sleeping for 10 ms.
[2024-02-08 10:33:19.723]: (INFO) sleeping for 10 ms.
----------------------------------------------------------------------------------------------------
[2024-02-08 10:33:19.733]: (INFO) Perf test completed successfully!
[2024-02-08 10:33:19.733]: (INFO) total_messages=[32],elapsed_time=[0.13s]
[2024-02-08 10:33:19.733]: (INFO) messages_per_second=[253],average_latency=[23006.15µs]
----------------------------------------------------------------------------------------------------
Latency Histogram
----------------------------------------------------------------------------------------------------
[100µs   - 110µs  ]======> 2 (6.25%)
[110µs   - 120µs  ]=========> 3 (9.38%)
[120µs   - 130µs  ]======> 2 (6.25%)
[140µs   - 150µs  ]===> 1 (3.12%)
[200µs   - 210µs  ]===> 1 (3.12%)
[220µs   - 230µs  ]===> 1 (3.12%)
[240µs   - 250µs  ]===> 1 (3.12%)
[250µs   - 260µs  ]===> 1 (3.12%)
[270µs   - 280µs  ]===> 1 (3.12%)
[280µs   - 290µs  ]======> 2 (6.25%)
[310µs   - 320µs  ]===> 1 (3.12%)
[44290µs - 44300µs]===> 1 (3.12%)
[44350µs - 44360µs]===> 1 (3.12%)
[44950µs - 44960µs]===> 1 (3.12%)
[44970µs - 44980µs]===> 1 (3.12%)
[45070µs - 45080µs]===> 1 (3.12%)
[45080µs - 45090µs]===> 1 (3.12%)
[45140µs - 45150µs]===> 1 (3.12%)
[45150µs - 45160µs]===> 1 (3.12%)
[45660µs - 45670µs]===> 1 (3.12%)
[45690µs - 45700µs]===> 1 (3.12%)
[45740µs - 45750µs]===> 1 (3.12%)
[47270µs - 47280µs]===> 1 (3.12%)
[47280µs - 47290µs]===> 1 (3.12%)
[47320µs - 47330µs]===> 1 (3.12%)
[47350µs - 47360µs]===> 1 (3.12%)
[47750µs - 47760µs]===> 1 (3.12%)
----------------------------------------------------------------------------------------------------
```

To build evh_perf, ensure you are in <project_subdirectory>/etc/evh_perf and then execute:

```text
$ cargo build --release
```

To run evh_perf:

```text
$ ./target/release/evh_perf --help
```

This will list the options.

