#/bin/bash
declare -a pixbuf_loaders=(
"pixbufloader_svg.dll"
)
 
SCHEMAS="share/glib-2.0/schemas"
PIXBUF="lib/gdk-pixbuf-2.0/2.10.0/loaders"
 
copy_libs() {
   cp $(ldd $1 | grep -Po "$MSYSTEM_PREFIX/[^ ]*" | sort -u) .
}
 
cd $(dirname "$0")
 
echo Building
cargo build -r
 
echo Creating pack dir
rm -rf pack
mkdir pack
cp target/release/tf2-demo-player.exe pack/
cd pack
 
echo Copying libraries
copy_libs tf2-demo-player.exe
 
echo Creating glib schemas
mkdir -p $SCHEMAS
glib-compile-schemas --targetdir $SCHEMAS $MSYSTEM_PREFIX/$SCHEMAS
 
echo Copying pixbuf loaders
mkdir -p $PIXBUF
for loader in "${pixbuf_loaders[@]}"; do
    cp $MSYSTEM_PREFIX/$PIXBUF/$loader $PIXBUF
done
for loader in "$PIXBUF"/*; do
    copy_libs $loader
done
export GDK_PIXBUF_MODULEDIR=$PIXBUF
gdk-pixbuf-query-loaders > $PIXBUF/../loaders.cache
