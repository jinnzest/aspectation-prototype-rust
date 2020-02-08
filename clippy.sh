 function pause(){
   read -p "$*"
}

file="./build.properties"

if [ -f "$file" ]
then
    echo "$file found."
 . $file

LLVM_SYS_80_PREFIX=$llvm_path cargo clippy
else
    echo "$file not found."
fi
