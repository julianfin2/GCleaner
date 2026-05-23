# GCleaner

GCleaner 是一个用于批量清理 Google 账号数据的桌面工具。

## 功能

- 批量导入短期 Google access token
- 自动识别 token 对应账号、有效期和权限范围
- 扫描并删除“我的云端硬盘”顶层文件和目录
- 移除“与我共享”中文件对当前账号的直接权限
- 根据 Gmail 中的 Drive 文件邮件线索清理权限，并删除相关邮件
- 清理 Google 通讯录中的联系人
- 清理 Google Tasks 中的任务和任务列表

## 使用说明

1. 获取包含所需权限范围的 Google access token。
2. 在“账号授权”页面中粘贴 token，每行一个。
3. 导入后确认账号和剩余有效时间。
4. 在左侧选择对应清理功能，先扫描，再确认执行清理。

access token 通常只有短期有效期，过期后需要重新导入。若 token 缺少某项 API 权限，对应功能会提示权限不足。

所需权限：

```txt
https://www.googleapis.com/auth/drive
https://mail.google.com/
https://www.googleapis.com/auth/contacts
https://www.googleapis.com/auth/tasks
https://www.googleapis.com/auth/userinfo.email
openid
```

## 注意事项

- 删除和权限移除操作可能不可恢复，请在执行前确认扫描结果。
- “移除共享”只会处理明确授予当前账号的直接权限，公开链接、群组、域或继承权限不会被贸然移除。
- “清理联系人”只处理通讯录联系人，不处理“其他联系人”。
- “清理任务”会删除用户创建的任务列表，并清空默认任务列表中的任务。
