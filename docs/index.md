* [Code Coverage](https://cgilliard.github.io/bitcoinmw/code_coverage.html)
* Build Information
    - Linux dependencies (ubuntu minimal):
```
# sudo apt update
# sudo apt install git libtool make clang rustc -y
# sudo apt-get install -y libgmp-dev automake autoconf
# git clone https://github.com/cgilliard/bitcoinmw
# cd bitcoinmw
# ./build
```
    - Test
```
# ./build clean
# ./test
```
        
