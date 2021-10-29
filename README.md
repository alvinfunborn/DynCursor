# DynCursor
## 动态修改鼠标指针光标图标样式, 根据键盘布局/语言/输入法的切换而变化.

假如在Windows上安装了多门语言频繁切换, 同时隐藏了语言栏, 使用时可能很难感知当前输入法是什么语言, 需要查看任务栏语言图标或者直接打字(薛定谔的输入法)来判断, 影响心态;

此工具能帮助快速判断所用的语言, 通过简单几步配置, 即可在使用英文时看到鼠标白色指针, 使用中文时看到鼠标黑色指针; 也可以[下载](http://www.rw-designer.com/cursor-library/set-0)或自制其他颜色和图标的指针;

实现思路是一直查询当前使用的语言, 如果当前语言是`中文(简体,中国)`, 则替换鼠标指针的状态
1. `正常选择`替换为`帮助选择`
1. `文本选择`替换为`候选`
1. `链接选择`替换为`不可用`

默认替换为`帮助选择`和`候选`和`不可用`是因为这些指针状态极少出现, 或者可以在[配置文件](config/config.yaml)中修改成其他替换指针

- 支持Windows10/11日常办公, 未测试其他平台和软件; 
- 仅支持语言切换(无法监听`shift`切换输入法, 使用`win+space`或`alt+shift`来切换语言);

## 使用
1. 下载[DynCursor.zip](https://github.com/alvinfunborn/DynCursor/releases), 解压到任意目录, 无需安装
1. 按需修改配置文件[config/config.yaml](config/config.yaml)
    > 默认配置是在`中文(简体,中国)`时修改`标准箭头`和`文本选择`和`链接选择`的样式; 其他光标保持不变, 但`帮助选择`和`候选`和`不可用`分别会变成`标准箭头`和`文本选择`和`链接选择`
1. 建议修改`鼠标属性-指针`自定义里的图标, 会有更好的观感
    1. 修改`帮助选择`样式
        1. 选中`帮助选择`
        ![avatar](resources/before-a.jpg)
        1. 浏览可用图标, 可以[下载](http://www.rw-designer.com/cursor-library/set-0)或自制cur文件
        ![avatar](resources/search-a.jpg)
        1. 确认
        ![avatar](resources/after-a.jpg)
    1. 修改`候选`样式
        1. 选中`候选`
        ![avatar](resources/before-i.jpg)
        1. 浏览可用图标, 可以[下载](http://www.rw-designer.com/cursor-library/set-0)或自制cur文件
        ![avatar](resources/search-i.jpg)
        1. 确认
        ![avatar](resources/after-i.jpg)
    1. 修改`不可用`样式
        1. 选中`不可用`
        ![avatar](resources/before-h.jpg)
        1. 浏览可用图标, 可以[下载](http://www.rw-designer.com/cursor-library/set-0)或自制cur文件
        ![avatar](resources/search-h.jpg)
        1. 确认
        ![avatar](resources/after-h.jpg)
    1. 应用
1. 建议关闭`鼠标属性-指针选项-在打字时隐藏指针`
1. 建议打开`鼠标属性-指针选项-当按CTRL键时显示指针的位置`
1. 如果没有安装英语语言, 建议在`设置-语言`添加语言`英语(美国)`
1. 启动DynCursor.exe, 这是一个无窗体程序, 启动后切换输入法来查看效果
1. 如果要停止运行, 打开任务管理器找到`DynCursor.exe`, 结束任务
1. 如果要重置鼠标指针, 在`鼠标属性-指针`中选择任一方案应用

## 效果图
![avatar](resources/finalcut.gif)

## 开机自启动
[在 Windows 中更改哪些应用在启动时自动运行](https://support.microsoft.com/zh-cn/windows/%E5%9C%A8-windows-%E4%B8%AD%E6%9B%B4%E6%94%B9%E5%93%AA%E4%BA%9B%E5%BA%94%E7%94%A8%E5%9C%A8%E5%90%AF%E5%8A%A8%E6%97%B6%E8%87%AA%E5%8A%A8%E8%BF%90%E8%A1%8C-9115d841-735e-488d-e749-9ba301d441e6#ID0EBD=Windows_10)

## 附录
### 常用语言编码
语言|编码
-|-
中文(简体,中国)|0804
中文(繁体,中国台湾)|0404
中文(繁体,中国香港)|0C04
中文(繁体,中国澳门)|1404
英语(美国)|0409
英语(英国)|0809

更多编码参考[Windows Language Code Identifier (LCID) Reference](https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-lcid/70feba9f-294e-491e-b6eb-56532684c37f)

### 指针状态编码
状态|样式|编码
-|-|-
正常选择|标准箭头|32512
帮助选择|标准箭头+问号|32651
后台运行|标准箭头+小沙漏|32650
忙|沙漏|32514
精确选择|十字|32515
文本选择|I|32513
不可用|斜线圆圈|32648
垂直调整大小|南北向双向箭头|32654
水平调整大小|东西向双向箭头|32644
沿对角线调整大小1|西北-东南的双向箭头|32642
沿对角线调整大小2|东北-西南的双向箭头|32643
移动|四向箭头|32646
候选|向上的箭头|32516
链接选择|手指|32649

更多编码参考[The name of the cursor resource](https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadcursorw)
