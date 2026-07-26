# fd

[![CICD](https://github.com/sharkdp/fd/actions/workflows/CICD.yml/badge.svg)](https://github.com/sharkdp/fd/actions/workflows/CICD.yml)
[![Version info](https://img.shields.io/crates/v/fd-find.svg)](https://crates.io/crates/fd-find)
[[中文](https://github.com/cha0ran/fd-zh)]
[[한국어](https://github.com/spearkkk/fd-kor)]
[[Tiếng Việt](README_vi.md)]

`fd` là một chương trình tìm kiếm các mục trong hệ thống tập tin của bạn.
Đây là một lựa chọn thay thế đơn giản, nhanh chóng và thân thiện với người dùng cho [`find`](https://www.gnu.org/software/findutils/).
Tuy không nhằm hỗ trợ toàn bộ chức năng mạnh mẽ của `find`, nhưng nó cung cấp các thiết lập mặc định hợp lý
(theo quan điểm riêng) cho đa số các trường hợp sử dụng.

[Cài đặt](#cài-đặt) • [Cách sử dụng](#cách-sử-dụng) • [Xử lý sự cố](#xử-lý-sự-cố)

## Tính năng

* Cú pháp trực quan: `fd PATTERN` thay vì `find -iname '*PATTERN*'`.
* Hỗ trợ biểu thức chính quy (mặc định) và mẫu glob.
* [Rất nhanh](#đo-đạc) nhờ duyệt thư mục song song.
* Sử dụng màu sắc để làm nổi bật các loại tập tin khác nhau (giống như `ls`).
* Hỗ trợ [thực thi lệnh song song](#thực-thi-lệnh)
* Phân biệt chữ hoa/thường thông minh: mặc định tìm kiếm không phân biệt chữ hoa/thường. Nó tự chuyển
  sang phân biệt chữ hoa/thường nếu mẫu chứa ký tự
  viết hoa[\*](http://vimdoc.sourceforge.net/htmldoc/options.html#'smartcase').
* Mặc định bỏ qua các thư mục và tập tin ẩn.
* Mặc định bỏ qua các mẫu từ `.gitignore` của bạn.
* Tên lệnh *ngắn hơn 50%*[\*](https://github.com/ggreer/the_silver_searcher) so với
  `find` :-).

## Demo

![Demo](doc/screencast.svg)

## Cách sử dụng

Trước tiên, để có cái nhìn tổng quan về tất cả các tùy chọn dòng lệnh, bạn có thể chạy
[`fd -h`](#tùy-chọn-dòng-lệnh) để xem thông báo trợ giúp ngắn gọn hoặc `fd --help` để xem phiên bản chi tiết hơn.

### Tìm kiếm đơn giản

*fd* được thiết kế để tìm các mục trong hệ thống tập tin của bạn. Tìm kiếm cơ bản nhất bạn có thể thực hiện là
chạy *fd* với một đối số duy nhất: mẫu tìm kiếm. Ví dụ: giả sử bạn muốn tìm một
tập lệnh cũ (tên có chứa `netflix`):
``` bash
> fd netfl
Software/python/imdb-ratings/netflix-details.py
```
Nếu được gọi chỉ với một đối số duy nhất như thế này, *fd* sẽ tìm kiếm đệ quy thư mục hiện tại
cho bất kỳ mục nào *có chứa* mẫu `netfl`.

### Tìm kiếm bằng biểu thức chính quy

Mẫu tìm kiếm được xử lý như một biểu thức chính quy. Ở đây, chúng ta tìm các mục bắt đầu
bằng `x` và kết thúc bằng `rc`:
``` bash
> cd /etc
> fd '^x.*rc$'
X11/xinit/xinitrc
X11/xinit/xserverrc
```

Cú pháp biểu thức chính quy được sử dụng bởi `fd` được [mô tả tại đây](https://docs.rs/regex/latest/regex/#syntax).

### Chỉ định thư mục gốc

Nếu chúng ta muốn tìm kiếm trong một thư mục cụ thể, có thể truyền nó làm đối số thứ hai cho *fd*:
``` bash
> fd passwd /etc
/etc/default/passwd
/etc/pam.d/passwd
/etc/passwd
```

### Liệt kê tất cả tập tin, đệ quy

*fd* có thể được gọi mà không có đối số. Điều này rất hữu ích để có cái nhìn tổng quan nhanh về tất cả các mục
trong thư mục hiện tại, một cách đệ quy (tương tự `ls -R`):
``` bash
> cd fd/tests
> fd
testenv
testenv/mod.rs
tests.rs
```

Nếu bạn muốn sử dụng chức năng này để liệt kê tất cả tập tin trong một thư mục nhất định, bạn phải sử dụng
một mẫu bao quát như `.` hoặc `^`:
``` bash
> fd . fd/tests/
testenv
testenv/mod.rs
tests.rs
```

### Tìm kiếm theo phần mở rộng tập tin cụ thể

Thông thường, chúng ta quan tâm đến tất cả các tập tin của một loại cụ thể. Điều này có thể được thực hiện với tùy chọn `-e` (hoặc
`--extension`). Ở đây, chúng ta tìm kiếm tất cả các tập tin Markdown trong kho lưu trữ fd:
``` bash
> cd fd
> fd -e md
CONTRIBUTING.md
README.md
```

Tùy chọn `-e` có thể được sử dụng kết hợp với một mẫu tìm kiếm:
``` bash
> fd -e rs mod
src/fshelper/mod.rs
src/lscolors/mod.rs
tests/testenv/mod.rs
```

### Tìm kiếm theo tên tập tin chính xác

Để tìm các tập tin có tên khớp chính xác với mẫu được cung cấp, hãy sử dụng tùy chọn `-g` (hoặc `--glob`):
``` bash
> fd -g libc.so /usr
/usr/lib32/libc.so
/usr/lib/libc.so
```

### Tập tin ẩn và bị bỏ qua
Theo mặc định, *fd* không tìm kiếm trong các thư mục ẩn và không hiển thị các tập tin ẩn trong
kết quả tìm kiếm. Để tắt hành vi này, chúng ta có thể sử dụng tùy chọn `-H` (hoặc `--hidden`):
``` bash
> fd pre-commit
> fd -H pre-commit
.git/hooks/pre-commit.sample
```

Nếu chúng ta làm việc trong một thư mục là kho lưu trữ Git (hoặc có chứa kho lưu trữ Git), *fd* sẽ không
tìm kiếm trong các thư mục (và không hiển thị các tập tin) khớp với một trong các mẫu `.gitignore`. Để tắt
hành vi này, chúng ta có thể sử dụng tùy chọn `-I` (hoặc `--no-ignore`):
``` bash
> fd num_cpu
> fd -I num_cpu
target/debug/deps/libnum_cpus-f5ce7ef99006aa05.rlib
```

Để thực sự tìm kiếm *tất cả* tập tin và thư mục, chỉ cần kết hợp các tùy chọn ẩn và bỏ qua để hiển thị
mọi thứ (`-HI`) hoặc sử dụng `-u`/`--unrestricted`.

### Khớp toàn bộ đường dẫn
Theo mặc định, *fd* chỉ khớp tên tập tin của mỗi tập tin. Tuy nhiên, bằng cách sử dụng tùy chọn `--full-path` hoặc `-p`,
bạn có thể khớp với toàn bộ đường dẫn.

```bash
> fd -p -g '**/.git/config'
> fd -p '.*/lesson-\d+/[a-z]+.(jpg|png)'
```

### Thực thi lệnh

Thay vì chỉ hiển thị kết quả tìm kiếm, bạn thường muốn *làm gì đó* với chúng. `fd`
cung cấp hai cách để thực thi các lệnh bên ngoài cho mỗi kết quả tìm kiếm của bạn:

* Tùy chọn `-x`/`--exec` chạy một lệnh bên ngoài *cho mỗi kết quả tìm kiếm* (song song).
* Tùy chọn `-X`/`--exec-batch` khởi chạy lệnh bên ngoài một lần, với *tất cả kết quả tìm kiếm làm đối số*.

#### Ví dụ

Tìm đệ quy tất cả các kho lưu trữ zip và giải nén chúng:
``` bash
fd -e zip -x unzip
```
Nếu có hai tập tin như vậy, `file1.zip` và `backup/file2.zip`, lệnh này sẽ thực thi
`unzip file1.zip` và `unzip backup/file2.zip`. Hai tiến trình `unzip` chạy song song
(nếu các tập tin được tìm thấy đủ nhanh).

Tìm tất cả các tập tin `*.h` và `*.cpp` và định dạng tự động tại chỗ với `clang-format -i`:
``` bash
fd -e h -e cpp -x clang-format -i
```
Lưu ý cách tùy chọn `-i` của `clang-format` có thể được truyền như một đối số riêng biệt. Đây là lý do
chúng ta đặt tùy chọn `-x` ở cuối.

Bất kỳ đối số vị trí nào sau `-x` đều thuộc về mẫu lệnh, không phải của `fd`. Nếu bạn
cũng muốn truyền một mẫu hoặc đường dẫn tìm kiếm, hãy đặt `-x` ở cuối:
``` bash
fd pattern path -x echo
```

Tìm tất cả các tập tin `test_*.py` và mở chúng trong trình soạn thảo yêu thích của bạn:
``` bash
fd -g 'test_*.py' -X vim
```
Lưu ý rằng chúng ta sử dụng chữ `-X` hoa ở đây để mở một phiên bản `vim` duy nhất. Nếu có hai tập tin như vậy,
`test_basic.py` và `lib/test_advanced.py`, lệnh này sẽ chạy `vim test_basic.py lib/test_advanced.py`.

Để xem thông tin chi tiết như quyền tập tin, chủ sở hữu, kích thước tập tin, v.v., bạn có thể yêu cầu `fd` hiển thị chúng
bằng cách chạy `ls` cho mỗi kết quả:
``` bash
fd … -X ls -lhd --color=always
```
Mẫu này hữu ích đến nỗi `fd` cung cấp một lối tắt. Bạn có thể sử dụng tùy chọn `-l`/`--list-details`
để thực thi `ls` theo cách này: `fd … -l`.

Tùy chọn `-X` cũng hữu ích khi kết hợp `fd` với [ripgrep](https://github.com/BurntSushi/ripgrep/) (`rg`) để tìm kiếm trong một lớp tập tin nhất định, như tất cả các tập tin nguồn C++:
```bash
fd -e cpp -e cxx -e h -e hpp -X rg 'std::cout'
```

Chuyển đổi tất cả tập tin `*.jpg` thành tập tin `*.png`:
``` bash
fd -e jpg -x convert {} {.}.png
```
Ở đây, `{}` là một trình giữ chỗ cho kết quả tìm kiếm. `{.}` cũng tương tự, nhưng không có phần mở rộng tập tin.
Xem bên dưới để biết thêm chi tiết về cú pháp trình giữ chỗ.

Đầu ra của thiết bị đầu cuối từ các lệnh chạy song song sử dụng `-x` sẽ không bị đan xen hoặc xáo trộn,
vì vậy `fd -x` có thể được sử dụng để song song hóa một tác vụ chạy trên nhiều tập tin một cách cơ bản.
Một ví dụ về điều này là tính toán checksum của từng tập tin riêng lẻ trong một thư mục.
```
fd -tf -x md5sum > file_checksums.txt
```

#### Cú pháp trình giữ chỗ

Các tùy chọn `-x` và `-X` sử dụng một *mẫu lệnh* dưới dạng một chuỗi các đối số (thay vì một chuỗi đơn).
Nếu bạn muốn thêm các tùy chọn bổ sung vào `fd` sau mẫu lệnh, bạn có thể kết thúc nó bằng `\;`.

Ví dụ: `fd -x echo \; pattern path` coi `pattern path` là các đối số của `fd` thay vì
truyền chúng cho `echo`. Trong thực tế, thường rõ ràng hơn khi viết `fd pattern path -x echo`.

Cú pháp tạo lệnh tương tự như của [GNU Parallel](https://www.gnu.org/software/parallel/):

- `{}`: Trình giữ chỗ sẽ được thay thế bằng đường dẫn của kết quả tìm kiếm
  (`documents/images/party.jpg`).
- `{.}`: Giống như `{}`, nhưng không có phần mở rộng tập tin (`documents/images/party`).
- `{/}`: Trình giữ chỗ sẽ được thay thế bằng tên cơ sở của kết quả tìm kiếm (`party.jpg`).
- `{//}`: Thư mục cha của đường dẫn được tìm thấy (`documents/images`).
- `{/.}`: Tên cơ sở, đã loại bỏ phần mở rộng (`party`).

Nếu bạn không bao gồm trình giữ chỗ, *fd* sẽ tự động thêm `{}` vào cuối.

#### Thực thi song song so với tuần tự

Đối với `-x`/`--exec`, bạn có thể kiểm soát số lượng tác vụ song song bằng tùy chọn `-j`/`--threads`.
Sử dụng `--threads=1` để thực thi tuần tự.

### Loại trừ các tập tin hoặc thư mục cụ thể

Đôi khi chúng ta muốn bỏ qua kết quả tìm kiếm từ một thư mục con cụ thể. Ví dụ: chúng ta có thể
muốn tìm kiếm tất cả các tập tin và thư mục ẩn (`-H`) nhưng loại trừ tất cả các kết quả khớp từ thư mục
`.git`. Chúng ta có thể sử dụng tùy chọn `-E` (hoặc `--exclude`) cho việc này. Nó nhận một mẫu glob
bất kỳ làm đối số:
``` bash
> fd -H -E .git …
```

Chúng ta cũng có thể sử dụng điều này để bỏ qua các thư mục được gắn kết:
``` bash
> fd -E /mnt/external-drive …
```

.. hoặc để bỏ qua một số loại tập tin nhất định:
``` bash
> fd -E '*.bak' …
```

Để làm cho các mẫu loại trừ này trở nên vĩnh viễn, bạn có thể tạo một tập tin `.fdignore`. Chúng hoạt động giống như
các tập tin `.gitignore`, nhưng dành riêng cho `fd`. Ví dụ:
``` bash
> cat ~/.fdignore
/mnt/external-drive
*.bak
```

> [!NOTE]
> `fd` cũng hỗ trợ các tập tin `.ignore` được sử dụng bởi các chương trình khác như `rg` hoặc `ag`.

Nếu bạn muốn `fd` bỏ qua các mẫu này trên toàn cầu, bạn có thể đặt chúng trong tập tin bỏ qua toàn cầu của `fd`.
Tập tin này thường nằm ở `~/.config/fd/ignore` trên macOS hoặc Linux, và `%APPDATA%\fd\ignore` trên
Windows.

Bạn có thể muốn bao gồm `.git/` trong tập tin `fd/ignore` của mình để các thư mục `.git` và nội dung của chúng
không được bao gồm trong đầu ra nếu bạn sử dụng tùy chọn `--hidden`.

### Xóa tập tin

Bạn có thể sử dụng `fd` để xóa tất cả các tập tin và thư mục khớp với mẫu tìm kiếm của bạn.
Nếu bạn chỉ muốn xóa tập tin, bạn có thể sử dụng tùy chọn `--exec-batch`/`-X` để gọi `rm`. Ví dụ,
để xóa đệ quy tất cả các tập tin `.DS_Store`, chạy:
``` bash
> fd -H '^\.DS_Store$' -tf -X rm
```
Nếu bạn không chắc chắn, hãy luôn gọi `fd` mà không có `-X rm` trước. Ngoài ra, hãy sử dụng tùy chọn "tương tác"
của `rm`:
``` bash
> fd -H '^\.DS_Store$' -tf -X rm -i
```

Nếu bạn cũng muốn xóa một loại thư mục nhất định, bạn có thể sử dụng kỹ thuật tương tự. Bạn sẽ
phải sử dụng cờ `--recursive`/`-r` của `rm` để xóa các thư mục.

> [!NOTE]
> Có những tình huống mà việc sử dụng `fd … -X rm -r` có thể gây ra điều kiện tranh chấp: nếu bạn có một
> đường dẫn như `…/foo/bar/foo/…` và muốn xóa tất cả các thư mục tên `foo`, bạn có thể kết thúc trong tình huống
> thư mục `foo` bên ngoài bị xóa trước, dẫn đến lỗi *"'foo/bar/foo': Không có tập tin hoặc thư mục như vậy"*
> (vô hại) trong lệnh gọi `rm`.

### Tùy chọn dòng lệnh

Đây là đầu ra của `fd -h`. Để xem đầy đủ các tùy chọn dòng lệnh, hãy sử dụng `fd --help` cũng
bao gồm văn bản trợ giúp chi tiết hơn nhiều.

```
Usage: fd [OPTIONS] [pattern [path]...]

Arguments:
  [pattern]  the search pattern (a regular expression, unless '--glob' is used; optional)
  [path]...  the root directories for the filesystem search (optional)

Options:
  -H, --hidden                     Search hidden files and directories
  -I, --no-ignore                  Do not respect .(git|fd)ignore files
  -s, --case-sensitive             Case-sensitive search (default: smart case)
  -i, --ignore-case                Case-insensitive search (default: smart case)
  -g, --glob                       Glob-based search (default: regular expression)
  -a, --absolute-path              Show absolute instead of relative paths
  -l, --list-details               Use a long listing format with file metadata
  -L, --follow                     Follow symbolic links
  -p, --full-path                  Search full abs. path (default: filename only)
  -d, --max-depth <depth>          Set maximum search depth (default: none)
  -E, --exclude <glob>             Exclude entries that match the given glob pattern
  -t, --type <filetype>            Filter by type: file (f), directory (d/dir), symlink (l),
                                   executable (x), empty (e), socket (s), pipe (p), char-device
                                   (c), block-device (b)
  -e, --extension <ext>            Filter by extension
  -S, --size <size>                Limit results based on the size of files
      --changed-within <date|dur>  Filter by file modification time (newer than)
      --changed-before <date|dur>  Filter by file modification time (older than)
  -o, --owner <user:group>         Filter by owning user and/or group
      --format <fmt>               Print results according to template
  -x, --exec <cmd>...              Execute a command for each search result
  -X, --exec-batch <cmd>...        Execute a command with all search results at once
  -c, --color <when>               When to use colors [default: auto] [possible values: auto,
                                   always, never]
      --hyperlink[=<when>]         Add hyperlinks to output paths [default: never] [possible
                                   values: auto, always, never]
      --ignore-contain <name>      Ignore directories containing the named entry
  -h, --help                       Print help (see more with '--help')
  -V, --version                    Print version
```

Lưu ý rằng các tùy chọn cũng có thể được đặt sau mẫu và/hoặc đường dẫn.

## Đo đạc

Hãy tìm kiếm trong thư mục home của tôi các tập tin kết thúc bằng `[0-9].jpg`. Nó chứa khoảng ~750.000
thư mục con và khoảng 4 triệu tập tin. Để tính trung bình và phân tích thống kê, tôi đang sử dụng
[hyperfine](https://github.com/sharkdp/hyperfine). Các đo đạc sau được thực hiện
với bộ nhớ đệm đĩa "ấm"/đã được nạp trước (kết quả cho bộ nhớ đệm đĩa "lạnh" cho thấy xu hướng tương tự).

Hãy bắt đầu với `find`:
```
Benchmark 1: find ~ -iregex '.*[0-9]\.jpg$'
  Time (mean ± σ):     19.922 s ±  0.109 s
  Range (min … max):   19.765 s … 20.065 s
```

`find` nhanh hơn nhiều nếu nó không cần thực hiện tìm kiếm biểu thức chính quy:
```
Benchmark 2: find ~ -iname '*[0-9].jpg'
  Time (mean ± σ):     11.226 s ±  0.104 s
  Range (min … max):   11.119 s … 11.466 s
```

Bây giờ hãy thử tương tự với `fd`. Lưu ý rằng `fd` thực hiện tìm kiếm biểu thức chính quy
theo mặc định. Tùy chọn `-u`/`--unrestricted` là cần thiết ở đây để
so sánh công bằng. Nếu không, `fd` sẽ không phải duyệt qua các thư mục ẩn và
các đường dẫn bị bỏ qua (xem bên dưới):
```
Benchmark 3: fd -u '[0-9]\.jpg$' ~
  Time (mean ± σ):     854.8 ms ±  10.0 ms
  Range (min … max):   839.2 ms … 868.9 ms
```
Đối với ví dụ cụ thể này, `fd` nhanh hơn khoảng **23 lần** so với `find -iregex`
và nhanh hơn khoảng **13 lần** so với `find -iname`. Nhân tiện, cả hai công cụ đều tìm thấy cùng một
546 tập tin :smile:.

**Lưu ý**: Đây là *một* đo đạc cụ thể trên *một* máy tính cụ thể. Mặc dù chúng tôi đã
thực hiện nhiều thử nghiệm khác nhau (và tìm thấy kết quả nhất quán), mọi thứ có thể
khác đối với bạn! Chúng tôi khuyến khích mọi người tự thử nghiệm. Xem
[kho lưu trữ này](https://github.com/sharkdp/fd-benchmarks) để biết tất cả các tập lệnh cần thiết.

Về tốc độ của *fd*, phần lớn công lao thuộc về các crate `regex` và `ignore` cũng được
sử dụng trong [ripgrep](https://github.com/BurntSushi/ripgrep) (hãy xem thử!).

## Xử lý sự cố

### `fd` không tìm thấy tập tin của tôi!

Hãy nhớ rằng `fd` mặc định bỏ qua các thư mục và tập tin ẩn. Nó cũng bỏ qua các mẫu
từ các tập tin `.gitignore`. Nếu bạn muốn chắc chắn tìm thấy mọi tập tin có thể, hãy luôn
sử dụng tùy chọn `-u`/`--unrestricted` (hoặc `-HI` để bật tập tin ẩn và bị bỏ qua):
``` bash
> fd -u …
```

Cũng hãy nhớ rằng theo mặc định, `fd` chỉ tìm kiếm dựa trên tên tập tin và
không so sánh mẫu với toàn bộ đường dẫn. Nếu bạn muốn tìm kiếm dựa trên toàn bộ
đường dẫn (tương tự như tùy chọn `-path` của `find`), bạn cần sử dụng tùy chọn `--full-path`
(hoặc `-p`).

### Đầu ra có màu sắc

`fd` có thể tô màu tập tin theo phần mở rộng, giống như `ls`. Để điều này hoạt động, biến môi trường
[`LS_COLORS`](https://linux.die.net/man/5/dir_colors) phải được thiết lập. Thông thường, giá trị
của biến này được thiết lập bởi lệnh `dircolors`, cung cấp một định dạng cấu hình thuận tiện
để định nghĩa màu sắc cho các định dạng tập tin khác nhau.
Trên hầu hết các bản phân phối, `LS_COLORS` đã được thiết lập sẵn. Nếu bạn đang dùng Windows hoặc đang tìm kiếm
các biến thể thay thế, đầy đủ hơn (hoặc nhiều màu sắc hơn), hãy xem [tại đây](https://github.com/sharkdp/vivid),
[tại đây](https://github.com/seebi/dircolors-solarized) hoặc
[tại đây](https://github.com/trapd00r/LS_COLORS).

`fd` cũng tôn trọng biến môi trường [`NO_COLOR`](https://no-color.org/).

### `fd` dường như không hiểu đúng mẫu regex của tôi

Nhiều ký tự đặc biệt trong regex (như `[]`, `^`, `$`, ..) cũng là các ký tự đặc biệt trong
shell của bạn. Nếu nghi ngờ, hãy luôn đặt mẫu regex trong dấu nháy đơn:

``` bash
> fd '^[A-Z][0-9]+$'
```

Nếu mẫu của bạn bắt đầu bằng dấu gạch ngang, bạn phải thêm `--` để báo hiệu kết thúc các tùy chọn
dòng lệnh. Nếu không, mẫu sẽ được hiểu như một tùy chọn dòng lệnh. Ngoài ra,
hãy sử dụng một lớp ký tự với một ký tự gạch ngang duy nhất:

``` bash
> fd -- '-pattern'
> fd '[-]pattern'
```

### "Không tìm thấy lệnh" cho các `alias` hoặc hàm shell

Các `alias` và hàm shell không thể được sử dụng để thực thi lệnh qua `fd -x` hoặc
`fd -X`. Trong `zsh`, bạn có thể làm cho alias trở thành toàn cục qua `alias -g myalias="…"`. Trong
`bash`, bạn có thể sử dụng `export -f my_function` để làm cho nó khả dụng cho các tiến trình con. Bạn vẫn
cần gọi `fd -x bash -c 'my_function "$1"' bash`. Đối với các trường hợp sử dụng khác hoặc shell khác, hãy sử dụng
một tập lệnh shell (tạm thời).

### Trình giữ chỗ trong `-x`/`-X`

Tùy thuộc vào shell của bạn, bạn có thể cần đặt dấu ngoặc kép cho các trình giữ chỗ (`{}`, `{/}`, `{//}`,
`{.}`, `{/.}`) để ngăn shell diễn giải chúng trước khi `fd` nhìn thấy chúng.

## Tích hợp với các chương trình khác

### Sử dụng fd với `fzf`

Bạn có thể sử dụng *fd* để tạo đầu vào cho công cụ tìm kiếm mờ dòng lệnh [fzf](https://github.com/junegunn/fzf):
``` bash
export FZF_DEFAULT_COMMAND='fd --type file'
export FZF_CTRL_T_COMMAND="$FZF_DEFAULT_COMMAND"
```

Sau đó, bạn có thể gõ `vim <Ctrl-T>` trên terminal để mở fzf và tìm kiếm qua các kết quả fd.

Ngoài ra, bạn có thể muốn theo dõi các liên kết tượng trưng và bao gồm các tập tin ẩn (nhưng loại trừ thư mục `.git`):
``` bash
export FZF_DEFAULT_COMMAND='fd --type file --follow --hidden --exclude .git'
```

Bạn thậm chí có thể sử dụng đầu ra có màu của fd bên trong fzf bằng cách đặt:
``` bash
export FZF_DEFAULT_COMMAND="fd --type file --color=always"
export FZF_DEFAULT_OPTS="--ansi"
```

Để biết thêm chi tiết, hãy xem [phần Mẹo](https://github.com/junegunn/fzf#tips) của README fzf.

### Sử dụng fd với `rofi`

[*rofi*](https://github.com/davatorium/rofi) là một ứng dụng menu khởi chạy đồ họa có khả năng tạo menu bằng cách đọc từ *stdin*. Chuyển đầu ra của `fd` vào chế độ `-dmenu` của `rofi` sẽ tạo ra các danh sách tập tin và thư mục có thể tìm kiếm mờ.

#### Ví dụ

Tạo một danh sách đa lựa chọn có thể tìm kiếm không phân biệt chữ hoa/thường của các tập tin *PDF* trong thư mục `$HOME` của bạn và mở lựa chọn với trình xem PDF đã cấu hình. Để liệt kê tất cả các loại tập tin, hãy bỏ đối số `-e pdf`.

``` bash
fd --type f -e pdf . $HOME | rofi -keep-right -dmenu -i -p FILES -multi-select | xargs -I {} xdg-open {}
```

Để sửa đổi danh sách được hiển thị bởi rofi, hãy thêm các đối số vào lệnh `fd`. Để sửa đổi hành vi tìm kiếm của rofi, hãy thêm các đối số vào lệnh `rofi`.

### Sử dụng fd với `emacs`

Gói emacs [find-file-in-project](https://github.com/technomancy/find-file-in-project) có thể
sử dụng *fd* để tìm tập tin.

Sau khi cài đặt `find-file-in-project`, thêm dòng `(setq ffip-use-rust-fd t)` vào
tập tin `~/.emacs` hoặc `~/.emacs.d/init.el` của bạn.

Trong emacs, chạy `M-x find-file-in-project-by-selected` để tìm các tập tin khớp. Ngoài ra, chạy
`M-x find-file-in-project` để liệt kê tất cả các tập tin có sẵn trong dự án.

### In đầu ra dưới dạng cây

Để định dạng đầu ra của `fd` dưới dạng cây tập tin, bạn có thể sử dụng lệnh `tree` với
`--fromfile`:
```bash
❯ fd | tree --fromfile
```

Điều này có thể hữu ích hơn so với chạy `tree` một mình vì `tree` không
bỏ qua bất kỳ tập tin nào theo mặc định, cũng không hỗ trợ nhiều tùy chọn phong phú như
`fd` để kiểm soát những gì được in:
```bash
❯ fd --extension rs | tree --fromfile
.
├── build.rs
└── src
    ├── app.rs
    └── error.rs
```

Trên bash và các shell tương tự, bạn có thể đơn giản tạo một alias:
```bash
❯ alias as-tree='tree --fromfile'
```

### Sử dụng fd với `xargs` hoặc `parallel`

Lưu ý rằng `fd` có tính năng tích hợp sẵn để [thực thi lệnh](#thực-thi-lệnh) với
các tùy chọn `-x`/`--exec` và `-X`/`--exec-batch`. Nếu bạn thích, bạn vẫn có thể sử dụng
nó kết hợp với `xargs`:
``` bash
> fd -0 -e rs | xargs -0 wc -l
```
Ở đây, tùy chọn `-0` yêu cầu *fd* phân tách kết quả tìm kiếm bằng ký tự NULL (thay vì
xuống dòng). Tương tự, tùy chọn `-0` của `xargs` yêu cầu nó đọc đầu vào theo cách này.

## Cài đặt

[![Packaging status](https://repology.org/badge/vertical-allrepos/fd-find.svg)](https://repology.org/project/fd-find/versions)

### Trên Ubuntu
*... và các bản phân phối Linux dựa trên Debian khác.*

Nếu bạn chạy Ubuntu 19.04 (Disco Dingo) hoặc mới hơn, bạn có thể cài đặt
[gói được duy trì chính thức](https://packages.ubuntu.com/fd-find):
```
apt install fd-find
```
Lưu ý rằng tệp nhị phân được gọi là `fdfind` vì tên nhị phân `fd` đã được sử dụng bởi một gói khác.
Nên sau khi cài đặt, bạn thêm một liên kết đến `fd` bằng cách thực thi lệnh
`ln -s $(which fdfind) ~/.local/bin/fd`, để sử dụng `fd` theo cách tương tự như trong tài liệu này.
Đảm bảo rằng `$HOME/.local/bin` nằm trong `$PATH` của bạn.

Nếu bạn sử dụng phiên bản Ubuntu cũ hơn, bạn có thể tải xuống gói `.deb` mới nhất từ
[trang phát hành](https://github.com/sharkdp/fd/releases) và cài đặt nó qua:
``` bash
dpkg -i fd_9.0.0_amd64.deb # điều chỉnh số phiên bản và kiến trúc
```

Lưu ý rằng các gói .deb trên trang phát hành cho dự án này vẫn đặt tên tệp thực thi là `fd`.

### Trên Debian

Nếu bạn chạy Debian Buster hoặc mới hơn, bạn có thể cài đặt
[gói Debian được duy trì chính thức](https://tracker.debian.org/pkg/rust-fd-find):
```
apt-get install fd-find
```
Lưu ý rằng tệp nhị phân được gọi là `fdfind` vì tên nhị phân `fd` đã được sử dụng bởi một gói khác.
Nên sau khi cài đặt, bạn thêm một liên kết đến `fd` bằng cách thực thi lệnh
`ln -s $(which fdfind) ~/.local/bin/fd`, để sử dụng `fd` theo cách tương tự như trong tài liệu này.
Đảm bảo rằng `$HOME/.local/bin` nằm trong `$PATH` của bạn.

Lưu ý rằng các gói .deb trên trang phát hành cho dự án này vẫn đặt tên tệp thực thi là `fd`.

### Trên Fedora

Bắt đầu từ Fedora 28, bạn có thể cài đặt `fd` từ các nguồn gói chính thức:
``` bash
dnf install fd-find
```

### Trên Alpine Linux

Bạn có thể cài đặt [gói fd](https://pkgs.alpinelinux.org/packages?name=fd)
từ các nguồn chính thức, với điều kiện bạn đã bật kho lưu trữ phù hợp:
```
apk add fd
```

### Trên Arch Linux

Bạn có thể cài đặt [gói fd](https://www.archlinux.org/packages/extra/x86_64/fd/) từ kho lưu trữ chính thức:
```
pacman -S fd
```
Bạn cũng có thể cài đặt fd [từ AUR](https://aur.archlinux.org/packages/fd-git).

### Trên Gentoo Linux

Bạn có thể sử dụng [ebuild fd](https://packages.gentoo.org/packages/sys-apps/fd) từ kho lưu trữ chính thức:
```
emerge -av fd
```

### Trên openSUSE Linux

Bạn có thể cài đặt [gói fd](https://software.opensuse.org/package/fd) từ kho lưu trữ chính thức:
```
zypper in fd
```

### Trên Void Linux

Bạn có thể cài đặt `fd` qua xbps-install:
```
xbps-install -S fd
```

### Trên ALT Linux

Bạn có thể cài đặt [gói fd](https://packages.altlinux.org/en/sisyphus/srpms/fd/) từ kho lưu trữ chính thức:
```
apt-get install fd
```

### Trên Solus

Bạn có thể cài đặt [gói fd](https://github.com/getsolus/packages/tree/main/packages/f/fd) từ kho lưu trữ chính thức:
```
eopkg install fd
```

### Trên RedHat Enterprise Linux (RHEL) 8/9/10, Almalinux 8/9/10, EuroLinux 8/9 hoặc Rocky Linux 8/9/10

Bạn có thể cài đặt [gói `fd`](https://copr.fedorainfracloud.org/coprs/tkbcopr/fd/) từ Fedora Copr.

```bash
dnf copr enable tkbcopr/fd
dnf install fd
```

Một phiên bản khác sử dụng bộ cấp phát bộ nhớ [chậm hơn](https://github.com/sharkdp/fd/pull/481#issuecomment-534494592) [thay vì jemalloc](https://bugzilla.redhat.com/show_bug.cgi?id=2216193#c1) cũng có sẵn từ kho EPEL8/9 với tên gói `fd-find`.

### Trên macOS

Bạn có thể cài đặt `fd` với [Homebrew](https://formulae.brew.sh/formula/fd):
```
brew install fd
```

… hoặc với MacPorts:
```
port install fd
```

### Trên Windows

Bạn có thể tải xuống các tệp nhị phân được biên dịch sẵn từ [trang phát hành](https://github.com/sharkdp/fd/releases).

Ngoài ra, bạn có thể cài đặt `fd` qua [Scoop](http://scoop.sh):
```
scoop install fd
```

Hoặc qua [Chocolatey](https://chocolatey.org):
```
choco install fd
```

Hoặc qua [Winget](https://learn.microsoft.com/en-us/windows/package-manager/):
```
winget install sharkdp.fd
```

### Trên GuixOS

Bạn có thể cài đặt [gói fd](https://guix.gnu.org/en/packages/fd-8.1.1/) từ kho lưu trữ chính thức:
```
guix install fd
```

### Trên Mise

Bạn có thể sử dụng [mise](https://github.com/jdx/mise) để cài đặt `fd` với lệnh như:
```
mise use -g fd@latest
```

### Trên NixOS / qua Nix

Bạn có thể sử dụng [trình quản lý gói Nix](https://nixos.org/nix/) để cài đặt `fd`:
```
nix-env -i fd
```

### Qua Flox

Bạn có thể sử dụng [Flox](https://flox.dev) để cài đặt `fd` vào môi trường Flox:
```
flox install fd
```

### Trên FreeBSD

Bạn có thể cài đặt [gói fd-find](https://www.freshports.org/sysutils/fd) từ kho lưu trữ chính thức:
```
pkg install fd-find
```

### Từ npm

Trên Linux và macOS, bạn có thể cài đặt gói [fd-find](https://npm.im/fd-find):

```
npm install -g fd-find
```

### Từ mã nguồn

Với trình quản lý gói Rust [cargo](https://github.com/rust-lang/cargo), bạn có thể cài đặt *fd* qua:
```
cargo install fd-find
```
Lưu ý rằng phiên bản rust *1.77.2* trở lên là bắt buộc.

`make` cũng cần thiết cho quá trình biên dịch.

### Từ tệp nhị phân

[Trang phát hành](https://github.com/sharkdp/fd/releases) bao gồm các tệp nhị phân được biên dịch sẵn cho Linux, macOS và Windows. Các tệp nhị phân tĩnh cũng có sẵn: hãy tìm các kho lưu trữ có chứa `musl` trong tên tập tin.

## Phát triển
```bash
git clone https://github.com/sharkdp/fd

# Biên dịch
cd fd
cargo build

# Chạy kiểm thử đơn vị và kiểm thử tích hợp
cargo test

# Cài đặt
cargo install --path .
```

### Hoàn tất (Completions)

#### Từ các kho lưu trữ phát hành

Các tập tin hoàn tất được xây dựng sẵn được bao gồm trong các kho lưu trữ phát hành (`.tar.gz`/`.zip`) trên
[trang Phát hành](https://github.com/sharkdp/fd/releases), trong thư mục `autocomplete`.
Để sử dụng các tập tin hoàn tất này:

- **bash**: Nạp tập tin `fd.bash` trong `~/.bashrc` của bạn, hoặc đặt nó vào một thư mục được nạp tự động.
- **zsh**: Di chuyển `_fd` vào một thư mục trong `fpath` của bạn (ví dụ: `~/.zfunc`).
- **fish**: Sao chép `fd.fish` vào `~/.config/fish/completions/`.
- **powershell**: Nạp `_fd.ps1` từ một trong các [tập lệnh hồ sơ](https://learn.microsoft.com/en-us/powershell/scripting/learn/shell/creating-profiles?view=powershell-7.5) của bạn.

#### Tạo từ fd

Bạn cũng có thể tạo các tập tin hoàn tất trực tiếp bằng `fd --gen-completions <shell>`:

```bash
# Bash
fd --gen-completions bash > ~/.local/share/bash-completion/completions/fd

# Zsh (đảm bảo ~/.zfunc nằm trong fpath của bạn)
fd --gen-completions zsh > ~/.zfunc/_fd

# Fish
fd --gen-completions fish > ~/.config/fish/completions/fd.fish

# PowerShell
fd --gen-completions powershell >> $PROFILE
```

## Người duy trì

- [sharkdp](https://github.com/sharkdp)
- [tmccombs](https://github.com/tmccombs)
- [tavianator](https://github.com/tavianator)

## Giấy phép

`fd` được phân phối theo các điều khoản của cả Giấy phép MIT và Giấy phép Apache 2.0.

Xem các tập tin [LICENSE-APACHE](LICENSE-APACHE) và [LICENSE-MIT](LICENSE-MIT) để biết chi tiết giấy phép.
