# 第三方组件与素材说明

DynCursor 的源码及原创文档采用根目录 [LICENSE](LICENSE) 中的 MIT 许可证。该授权不会替代第三方组件、Windows 界面元素及外部下载素材各自的授权。

## Rust 依赖

Windows x64 构建所使用的 Rust 组件及其声明的许可证、可取得的原始许可证文本，收录于 [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt)。其中也包含构建期间的依赖，便于核对。组件继续使用各自的许可证，不因被用于 DynCursor 而改用本项目许可证。

依赖未被本项目修改。MPL-2.0 组件 `selectors 0.38.0` 的完整源码（含原始逐文件声明）可从 [crates.io 下载](https://crates.io/api/v1/crates/selectors/0.38.0/download)，或访问对应 [上游修订](https://github.com/servo/stylo/tree/572ecba2d1600e7c3d490586692a209faf703baa/selectors)。MPL 文本及获取源码方式也包含在随包许可证汇编中。`typemap-ors` 和 `unsafe-any-ors` 上游仅声明 MIT、没有附带许可证文件，汇编明确标注并列出标准 MIT 条款，没有编造上游版权声明。

## 光标文件

以下历史文件在加入仓库时未记录足够的来源、作者和许可证信息，本项目不将其作为自有 MIT 素材重新授权：

- `resources/cur/*.cur`：`aero_arrow_glow.cur`、`aero_link_glow.cur`、`beam_glow.cur`。
- `resources/figure/*.cur`：`aero_arrow.cur`、`aero_helpsel.cur`、`aero_link.cur`、`aero_unavail.cur`、`arrow_r.cur`、`beam_r.cur`、`up_r.cur`。

历史 README 提供的下载站点为 [RealWorld Graphics 光标库](https://www.rw-designer.com/cursor-library/set-0)。该站点的[授权说明](https://www.rw-designer.com/licenses)明确指出，不同作者、不同作品可能使用不同授权，需要按具体作品页面核对。仅凭相同文件名或下载站点，无法确认仓库中的文件来自哪一个作品。

在来源和授权补齐前，这些文件的授权状态为**未核实**，不应将根目录 MIT 许可证解释为对这些图案的使用、修改或再分发授权。二次分发时，请改用自己制作或已取得明确授权的光标，并保留对应的许可信息。使用 DynCursor 程序本身不要求使用这些历史素材；可以在 Windows「鼠标属性 → 指针」中选择自己已有的光标文件。

**1.1.0 便携发布包不包含以上历史光标与截图。** `custom-arrow.cur`、`custom-beam.cur`、`custom-link.cur` 已在 1.1.0 替换成 DynCursor 原创图案，采用本项目 MIT License；SVG 原稿位于 `resources/cursor-source/`，由 `scripts/build-cursors.cjs` 生成。旧版本的同名文件不是这些新作品，不应追溯视为获得新授权。

## 截图与录屏

`resources/figure/*.jpg`、`resources/figure/*.gif` 及 Pages 中对应副本用于项目操作说明，其中可见的 Windows 界面、系统指针、第三方光标及其他标识仍属于各自权利人。本项目不对这些第三方元素重新授予 MIT 许可。

## 程序图标

1.1.0 的 Logo 为 DynCursor 原创矢量作品，原稿为 `resources/logo.svg`，采用本项目 MIT License。`scripts/build-icons.cjs` 统一导出程序、托盘、窗口和 Pages 的 PNG/ICO/SVG；没有引入第三方品牌或图标库。

## 设置窗口

窗口分组与托盘交互参考同一维护者的 ScreenBuoy 项目。DynCursor 的设置界面和光标管理逻辑在本仓库实现。

如能提供历史光标的原始下载页面、作者或授权凭据，请通过仓库 Issue 或 Pull Request 补充，便于逐个完善来源记录。
