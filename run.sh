function pause(){
   read -p "$*"
}

file="./build.properties"

if [ -f "$file" ]
then
    echo "$file found."
 . $file

LLVM_SYS_80_PREFIX=$llvm_path RUST_BACKTRACE=1 cargo run --verbose -- compile && printf "\nRunning generated executable...\n\n" && ./target/out
else
    echo "$file not found."
fi
