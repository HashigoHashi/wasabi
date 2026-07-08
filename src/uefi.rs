use crate::graphics::draw_font_fg;
use crate::graphics::Bitmap;
use crate::result::Result;
use core::fmt;
use core::mem::offset_of;
use core::mem::size_of;
use core::ptr::null_mut;

type EfiVoid = u8;
pub type EfiHandle = u64;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct EfiGuid {
    data0: u32,
    data1: u16,
    data2: u16,
    data3: [u8; 8],
}

const EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data0: 0x9042a9de,
    data1: 0x23dc,
    data2: 0x4a38,
    data3: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

const EFI_LOADED_IMAGE_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data0: 0x5B1B31A1,
    data1: 0x9562,
    data2: 0x11d2,
    data3: [0x8E, 0x3F, 0x00, 0xA0, 0xCA, 0x69, 0x72, 0x3B],
};
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[must_use]
#[repr(u64)] // この型はu64と全く同じレイアウト・サイズだ
pub enum EfiStatus {
    Success = 0,
}
/*
 * 0ってなんだ？？
 * let t = EfiStatus::Sucess;
 * のときにtはEfiStatus型の値だよね。0は何なの？？
 * 「Success = 0」は「代入」ではない。Successという列挙子に識別値0を割り当てる。
 * これがないとif status == EfiStatus::Success {...}のように書けない。
 */

#[repr(i64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
/*
 * 列挙型って理解しきれていない。
 * 中のメンバはすべてEfiMemoryType型としてあつかえる。
 * let t = EfiMemoryType::CONVENTIONAL_MEMORY;
 * このときtの型はEfiMemoryTypeであり、CONVENTIONAL_MEMORY型ではない。
 * enumはクラスのようなものではなく、シグナルのイメージ。
 * UEFIの仕様で
 * RESERVED               = 0
 * LOADER_CODE            = 1
 * LOADER_DATA            = 2
 * BOOT_SERVICES_CODE     = 3
 * ...
 * と決まっている。
 */
pub enum EfiMemoryType {
    RESERVED = 0,
    LOADER_CODE,
    LOADER_DATA,
    BOOT_SERVICES_CODE,
    BOOT_SERVICES_DATA,
    RUNTIME_SERVICES_CODE,
    RUNTIME_SERVICES_DATA,
    CONVENTIONAL_MEMORY,         // OSが自由に使える通常のメモリ
    UNUSABLE_MEMORY,
    ACPI_RECLAIM_MEMORY,
    ACPI_MEMORY_NVS,
    MEMORY_MAPPED_IO,
    MEMORY_MAPPED_IO_PORT_SPACE,
    PAL_CODE,
    PERSISTENT_MEMORY,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/*
 * 「ひとつの連続したメモリ領域の情報」
 *  メモリ区画
 */
pub struct EfiMemoryDescriptor {
    memory_type: EfiMemoryType, // メモリの用途
    physical_start: u64, //アドレスの開始位置
    virtual_start: u64,
    number_of_pages: u64, //ページ数
    attribute: u64,
}
/*
 * Type           = CONVENTIONAL_MEMORY
 * physical_start = 0x00100000
 * number_of_pages = 256
 * だった場合
 * 0x00100000
 * │
 * ├───────────────┐
 * │      256ページ分             │
 * └───────────────┘
 * という１つのメモリ領域を表す。
 * UEFIでは１ページ4096バイト（4KB）なので256 × 4096 = 1,048,576バイト（1MB）
 */
impl EfiMemoryDescriptor {
    pub fn memory_type(&self) -> EfiMemoryType {
        self.memory_type
    }
    pub fn number_of_pages(&self) -> u64 {
        self.number_of_pages
    }
    pub fn physical_start(&self) -> u64 {
        self.physical_start
    }
}

// スタックオーバーフロー対策のため、メモリマップのサイズを64KB固定
const MEMORY_MAP_BUFFER_SIZE: usize = 0x10000;

pub struct MemoryMapHolder {
    memory_map_buffer: [u8; MEMORY_MAP_BUFFER_SIZE], //メモリマップ本体(EfiMemoryDescriptorが連続している)
    /*
     * ん？？このu8の羅列に物理アドレスが羅列して入っているって感じ？？それの何が意味あるの？？
     * u8の配列には以下のようなデータがはいっている。
     * memory_map_buffer[0-39]：0x00000000 ～ 0x0009FFFF  CONVENTIONAL_MEMORY（使用可能）
     * memory_map_buffer[40-79]：0x000A0000 ～ 0x000FFFFF  RESERVED
     * memory_map_buffer[80-119]：0x00100000 ～ 0x03FFFFFF  BOOT_SERVICES_DATA
     * memory_map_buffer[120-159]：0x04000000 ～ 0x1FFFFFFF  CONVENTIONAL_MEMORY（使用可能）
     * memory_map_buffer[160-199]：0x20000000 ～ 0x20FFFFFF  ACPI_RECLAIM_MEMORY
     * これはEfiMemoryDescriptorの内容をそのまま広げたものである。
     * ではなぜ[EfiMemoryDescriptor; MEMORY_MAP_BUFFER_SIZE]にしないの？？
     * なんこになるかわからないから。
...
     */
    memory_map_size: usize, // メモリマップ全体のサイズ(バイト数)
    map_key: usize, // ExitBootServicesで使うキー
    descriptor_size: usize, // Descriptor１個のサイズ
    descriptor_version: u32, // Descriptorのバージョン
}
impl MemoryMapHolder {
    pub const fn new() -> MemoryMapHolder {
        MemoryMapHolder {
            memory_map_buffer: [0; MEMORY_MAP_BUFFER_SIZE],
            memory_map_size: MEMORY_MAP_BUFFER_SIZE,
            map_key: 0,
            descriptor_size: 0,
            descriptor_version: 0,
        }
    }
    pub fn iter(&self) -> MemoryMapIterator {
        MemoryMapIterator { map: self, ofs: 0 }
    }
}
impl Default for MemoryMapHolder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MemoryMapIterator<'a> {
    map: &'a MemoryMapHolder,
    ofs: usize,
}
impl<'a> Iterator for MemoryMapIterator<'a> {
    type Item = &'a EfiMemoryDescriptor;
    fn next(&mut self) -> Option<&'a EfiMemoryDescriptor> {
        if self.ofs >= self.map.memory_map_size {
            None
        } else {
            let e: &EfiMemoryDescriptor = unsafe {
                &*(self.map.memory_map_buffer.as_ptr().add(self.ofs)
                    as *const EfiMemoryDescriptor)
            };
            self.ofs += self.map.descriptor_size;
            Some(e)
        }
    }
}

#[repr(C)]
pub struct EfiBootServicesTable {
    _reserved0: [u64; 7],
    /*
     * _変数名ってなに？？なんで要素の数は7なの？？
     * _は「使われない変数」という意味。
     * UEFIがメモリ上に作成したEFI_BOOT_SERVICES構造体をRustでEfiBootServicesTableとして解釈するときに、
     * 先頭のu64×7個分の領域は今回はまとめてつかわないよ、ということ。
     */
    get_memory_map: extern "win64" fn(
        memory_map_size: *mut usize,
        /*
         * *ってなに？？ポインタ？？可変な参照という意味でいいのかな？？
         * *mutはCのポインタと同じように捉えて良い。
         * Rustでは&mut usizeという参照もあるがこれもおなじ参照である。しかしこちらはRustの安全なルールに従った参照である。
         */
        memory_map: *mut u8,
        map_key: *mut usize,
        descriptor_size: *mut usize,
        descriptor_version: *mut u32,
    ) -> EfiStatus,
    /*
     * これなに？？関数の定義？？
     * これは関数ポインタget_memory_mapの宣言。
     * Rustでは関数ポインタはlet f: fn(i32, i32) -> i32;のようにする。
     * extern "win64"は「この関数はWindows x64の呼び出し規約で呼び出す」という意味のお決まり文句。
     */
    _reserved2: [u64; 11],
    handle_protocol: extern "win64" fn(
        handle: EfiHandle,
        protocol: *const EfiGuid,
        interface: *mut *mut EfiVoid,
    ) -> EfiStatus,
    _reserved1: [u64; 9],
    exit_boot_services:
        extern "win64" fn(image_handle: EfiHandle, map_key: usize) -> EfiStatus,

    _reserved4: [u64; 10],
    locate_protocol: extern "win64" fn(
        protocol: *const EfiGuid,
        registration: *const EfiVoid,
        interface: *mut *mut EfiVoid,
    ) -> EfiStatus,
}
/*
 * EFIのAPI経由でメモリマップを受け取っている
 */
/*
 * get_memory_map関数はEFIがメモリ上に用意してくれるAPIなのになんでこれを定義しているの？？オーバーライド？？
 * オーバライドではなく、関数ポインタが指す関数を呼び出しやすいようにしている。引数をmapだけですむようにした。
 * このメソッドが存在しなければ
 * let services_table: EfiBootServicesTable;
 * services_table.get_memory_map(引数1, 引数2, ...);
 * とすると関数ポインタが指す関数を呼び出すことができるが、引数部分がかなり長くなってしまい呼び出しづらい。
 * ちなみに関数ポインタであることをわかりやすく書くなら、(services_tables.get_memory_map)(引数1, 引数2, ...)とすべき。
 * このメソッドを用意することで
 * (services_table.get_memory_map)に引数を渡す部分をラップできている。
 */
impl EfiBootServicesTable {
    pub fn get_memory_map(&self, map: &mut MemoryMapHolder) -> EfiStatus {
        (self.get_memory_map)(
            /*
             * ん？？何この書き方？？
             * (関数ポインタ)(引数)の呼び出しパターン。
             */
            &mut map.memory_map_size,
            /*
             * 関数の定義では引数は*mut 型名だったのに&mutでいいの？？Cのポインタを渡すところにRustにポインタを渡せるてこと？？
             * はい、できます。&mut T → *mut Tはできます。が逆はできません。
             * これはRustがCよりも安全な保証を持ったポインタだからできることです。
             * RustでOsをかける大きな理由の１つ。
             */
            map.memory_map_buffer.as_mut_ptr(),
            &mut map.map_key,
            &mut map.descriptor_size,
            &mut map.descriptor_version,
        )
        /*
         * あれ？？EfiStatusを返す処理が最後にないのはなぜ？？
         * UEIFはget_memory_mapで「64ビット整数」を返しているだけで、
         * Rustは#[repr(u64)] enum EfiStatusと宣言しているので、「この64ビット整数はEfiStatusとして扱ってよい」
         * つまり成功したときはUEFIは0を返すということ。
         * たとえばUEFIがEfiStatusに存在しない値を返した場合は、EfiStatusとして解釈できない。
         */
    }
}
const _: () = assert!(offset_of!(EfiBootServicesTable, get_memory_map) == 56);
const _: () = assert!(offset_of!(EfiBootServicesTable, exit_boot_services) == 232);
const _: () = assert!(offset_of!(EfiBootServicesTable, locate_protocol) == 320);

/*
 * UEFIはCで作った構造体なのに、なんでRustの構造体として受け取れるの？
 * #[repr(C)]をつければCの構造体とおなじメモリレイアウトになるのでCをRustで解釈できる。
 */
#[repr(C)]
pub struct EfiSystemTable {
    _reserved0: [u64; 12],
    pub boot_services: &'static EfiBootServicesTable,
    /*
     * &'staticって何？staticってライフタイムはどのくらいの期限を示している？？
     * プログラムの開始から終了まで生きているライフタイム
     * static x: i32 = 10;
     * なら
     * &xの型は&'static i32
     * つまりこの構造体をつくるにはstatci boot_services: EfiBootServicesTable = ...;がどこかでつくられないとだめだよね？？
     * 普通のプログラムならその考えであっている。ただこれはUEFIがつくっており、その参照をもらうだけでいい。
     */
}
const _: () = assert!(offset_of!(EfiSystemTable, boot_services) == 96);
/*
 * なにこれ？？
 * offset_of!(EfiSystemTable, boot_services)はEfiSystemTableの構造体の先頭から何バイト目にboot_servicesがあるか
 * u64が12個前にあるので96バイトboot_servicesの前にある。
 * assert!(96 == 96);となる。
 * 条件が偽ならコンパイル時にエラーになって止まってくれる安全装置。
 * Rustには「式」「文」「アイテム」があり、ファイルトップレベルにはアイテムしか書けない。
 * 基本的に「;」で終わるものは「文」という認識でよいが、「アイテム」であることもあるので注意が必要。
 * なのでassert!(...)という式はかけない。
 * const _: () = ...;は「名前のない定数をつくる」という「アイテム」
 */
impl EfiSystemTable {
    pub fn boot_services(&self) -> &EfiBootServicesTable {
        self.boot_services
    }
}

#[repr(C)]
#[derive(Debug)]
struct EfiGraphicsOutputProtocolPixelInfo {
    version: u32,
    pub horizontal_resolution: u32,  // 水平方向の画素数
    pub vertical_resolution: u32,    // 垂直方向の画素数
    _padding0: [u32; 5],
    pub pixels_per_scan_line: u32,   // 水平方向のデータに含まれる画素数。フレームバッファの一行あたりの画素数
}
/*
 * 1画素って何？？
 * 1画素＝4バイト(32ビット)です。
 * 1920×1080なら
 * horizontal_resolution = 1920
 * vertical_resolution   = 1080
 * なので
 * 1920×1020 = 2,073,600画素
 * あります。
 * 2,073,600×4
 * = 8,294,400バイト
 * = 8MB
 * のフレームバッファになる。
 */
const _: () = assert!(size_of::<EfiGraphicsOutputProtocolPixelInfo>() == 36);

#[repr(C)]
#[derive(Debug)]
struct EfiGraphicsOutputProtocolMode<'a> {
    pub max_mode: u32,
    pub mode:u32,
    pub info: &'a EfiGraphicsOutputProtocolPixelInfo,
    pub size_of_info: u64,
    pub frame_buffer_base: usize, // フレームバッファと呼ばれるメモリ領域の開始アドレス
    pub frame_buffer_size: usize // フレームバッファと呼ばれるメモリ領域の大きさ
}

#[repr(C)]
#[derive(Debug)]
struct EfiGraphicsOutputProtocol<'a> {
    _reserved: [u64; 3],
    pub mode: &'a EfiGraphicsOutputProtocolMode<'a>,
}

/*
 * 'aってやつはライフタイムパラメータってやつだよね。これって何がしたいの？？
 * 普通は引数の中のデータの参照を返すから、引数と同じライフタイムを返す感じじゃないの？？
 * fn foo<'a>(x: &'a T)->&'a T{...}
 * これは引数のライフタイムに依存せず、この関数内でライフタイムが決まるってものだね。
 * あれ、でもこの関数内ならこの関数終了時でライフタイムは終了しないか？？
 * そのとおりで
 * fn foo<'a>() -> &'a i32 {
 *   let x =10;
 *   &x
 *  }
 * は成り立たない。
 * 今回の関数においては関数内でUEFIが用意しているデータに対するポインタをもらってきているので関数終了時にデータは消えない。
 */
fn locate_graphic_protocol<'a>(
    efi_system_table: &EfiSystemTable,
) -> Result<&'a EfiGraphicsOutputProtocol<'a>> {
    /*
     * &'a EfiGraphicsOutputProtocol<'a> なんで'aがふたつもあるの？？
     * 後半の<'a>に関してはこのEfiGraphicsOutputProtocolという構造体がライフタイムパラメータを必要とする型だからである。
     * 構造体のメンバに絶対に同じスコープにないといけないメンバがあってそれと同じライフタイムにしている型という感じ。
     * &'aに関しては返す参照のライフタイムも'aであることを表している。
     */
    let mut graphic_output_protocol: *mut EfiGraphicsOutputProtocol<'_> = null_mut::<EfiGraphicsOutputProtocol>();
    /*
     * *mut 型名ってなに？？&mut 型名ならしっているけど、
     *「生ポインタ」
     * *mut T
     * 安全保障なし。unsafe必要。Cと同じ。
     * 
     *「参照」
     * &mut T
     * 安全保障あり。unsafe不要。Rust独自。
     * <'_>ってなに？？
     * 「EfiGraphicsOutputProtocol はライフタイムパラメータが必要な型だから、とりあえず適当に推論してください。」といういみ。
     */
    let status = (efi_system_table.boot_services.locate_protocol)(
        &EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID,
        null_mut::<EfiVoid>(),
        /*
         * null_mut::<型名>()ってなに？？
         * Cの
         * int *p;
         * のような宣言だとpはポインタで
         * 中身は
         * p
          ┌────────────┐
          │ ????????   │ ← ゴミ値（未初期化）
          └────────────┘
         * のような感じである。一方でRustの
         * let p = null_mut::<int>();
         * の初期化の場合は
         * p
          ┌────────────┐
          │ 0x000000   │ ← ゴミ値（未初期化）
          └────────────┘
         * となる。こうすることでUEFIがNULLだった場所に有効なアドレスを書き込んでくれる。
         * ちなみにRustでは未初期化変数をつかうことをコンパイラが禁止している
         * let p: *mut i32;
         * prinln!("{:?}", p); // コンパイルエラー
         */
        &mut graphic_output_protocol as *mut *mut EfiGraphicsOutputProtocol
            as *mut *mut EfiVoid,
        /*
         * これはなに？？
         * 分解して考えましょう
         * まずgraphic_output_protocolは*mut EfiGraphicsOutputProtocolです。
         * これに&mut *mut EfiGraphicsOutputProtocolとすることで
         * 「生ポインタへの可変な参照」をとることができます。(生ポインタの中身をNULLから書き換えるためでしょう...)
         * これをポインタのポインタ(参照の参照)という。
         * つぎにasはキャストです。「変数 as 型名」で任意の型名にキャストできます。
         * この場合
         * 「&mut *mut EfiGraphicsOutputProtocol　→ *mut *mut EfiGraphicsOutputProtocol」にキャストしています。
         * (おそらくCにRustの可変参照という概念がないからそうしているのでしょう...)
         * 最後のas *mut *mut EfiVoidの部分については型名がかわってしまっているけれどどういうこと？？
         * EfiVoidはu8のエイリアスです。ので実際には
         * *mut *mut u8にしようとしている。
         * *mut (EfiGraphicsOutputProtocolへのポインタ)を*mut (u8へのポインタ)としている。
         * Rust側ではgraphic_output_protocolはデリファレンスするとしっかりEfiGraphicsOutputProtocolとして扱える。
         * ただ渡されたCは「デリファレンスすると最終的にはu8があるんだよなー」ってならない？？
         * 実際にはEfiGraphicsOutputProtocolに相当するデータがあるのにこの場合はエラーにならないの？？
         * エラーにならない。渡された側はポインタのポインタを一回デリファレンスしてスタックに保持されているアドレスを書き換えることしか関心がない。
         */
    );
    if status != EfiStatus::Success {
        return Err("Failed to locate graphics output protocol");
    }
    Ok(unsafe { &*graphic_output_protocol })
    /*
     * なにこれ？？unsafe？？&*変数名も何？？
     * &*は生ポインタからRustの参照へ変換している。
       *mut EfiGraphicsOutputProtocol
      （生ポインタ）
        │
        │ *(デリファレンス)
        ▼
       EfiGraphicsOutputProtocol
      （ポインタが指す実体）
        │
        │ &
        ▼
      &EfiGraphicsOutputProtocol
     （Rustの安全な参照）
     * なるほど、ではなぜunsafeにしている？？もうすでに「生ポインタ」ではなく「参照」になっているじゃないか
     * unsafeは生ポインタを扱う処理を囲むものだから。この場合「生ポインタ」を「参照」に変換する処理が危険だから囲んでいる。
     */
}

pub struct EfiLoadedImageProtocol {
    _reserved: [u64; 8],
    pub image_base: u64,
    pub image_size: u64,
}

pub fn locate_loaded_image_protocol(
    image_handle: EfiHandle,
    efi_system_table: &EfiSystemTable,
) -> Result<&EfiLoadedImageProtocol> {
    let mut graphic_output_protocol = null_mut::<EfiLoadedImageProtocol>();
    let status = (efi_system_table.boot_services.handle_protocol)(
        image_handle,
        &EFI_LOADED_IMAGE_PROTOCOL_GUID,
        &mut graphic_output_protocol as *mut *mut EfiLoadedImageProtocol
            as *mut *mut EfiVoid,
    );
    if status != EfiStatus::Success {
        return Err("Failed to locate graphics output protocol");
    }
    Ok(unsafe { &*graphic_output_protocol })
}

/*
 * VRAMを表現する構造体
 */
#[derive(Clone, Copy)]
pub struct VramBufferInfo {
    buf: *mut u8, //フレームバッファの先頭アドレス
    width: i64,
    height: i64,
    pixels_per_line: i64,
}
impl Bitmap for VramBufferInfo { //トレイトをインターフェースとしているのね。まとまりのある関数を実装したいときにいいね。
    fn bytes_per_pixel(&self) -> i64 {
        4
    }
    fn pixels_per_line(&self) -> i64 {
        self.pixels_per_line
    }
    fn width(&self) -> i64 {
        self.width
    }
    fn height(&self) -> i64 {
        self.height
    }
    fn buf_mut(&mut self) -> *mut u8 {
        self.buf
    }
}

pub fn init_vram(efi_system_table: &EfiSystemTable) -> Result<VramBufferInfo> {
    let gp = locate_graphic_protocol(efi_system_table)?;
    Ok(VramBufferInfo {
        buf: gp.mode.frame_buffer_base as *mut u8,
        width: gp.mode.info.horizontal_resolution as i64,
        height: gp.mode.info.vertical_resolution as i64,
        pixels_per_line: gp.mode.info.pixels_per_scan_line as i64,
    })
}

pub struct VramTextWriter<'a> {
    vram: &'a mut VramBufferInfo,
    cursor_x: i64,
    cursor_y: i64,
}
impl<'a> VramTextWriter<'a> {
    pub fn new(vram: &'a mut VramBufferInfo) -> Self {
        Self {
            vram,
            cursor_x: 0,
            cursor_y: 0,
        }
    }
}
impl fmt::Write for VramTextWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if c == '\n' {
                self.cursor_y += 16;
                self.cursor_x = 0;
                continue;
            }
            draw_font_fg(self.vram, self.cursor_x, self.cursor_y, 0xffffff, c);
            self.cursor_x += 8;
        }
        Ok(())
    }
}

pub fn exit_from_efi_boot_services(
    image_handle: EfiHandle,
    efi_system_table: &EfiSystemTable,
    memory_map: &mut MemoryMapHolder,
) {
    /*
     * なぜループなの？？get_memory_mapとかは一回の実行で完結するはずだよね？？
     * 失敗することがあるため。
     * get_memory_mapで取得したmemory_mapがexit_boot_servicesを呼ぶ時点で、そのmap_keyが最新でないと失敗する。
     */
    loop {
        let status = efi_system_table.boot_services.get_memory_map(memory_map);
        assert_eq!(status, EfiStatus::Success);
        let status = (efi_system_table.boot_services.exit_boot_services)(
            image_handle,
            memory_map.map_key,
        );
        if status == EfiStatus::Success {
            break;
        }
    }
}