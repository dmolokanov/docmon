# docmon

docmon is a daemon for sending a live stream of container(s) resource usage statistics to Azure Log Analytics.

## Installation
```bash
sudo make install
```

After installation `docmon` requires to provide `CustomerId` and `SharedKey` into default config `/etc/docmon/config.toml`. These parameters available on `Azure Portal -> Log Analytics -> Advanced Settings` page.

```bash
vim /etc/docmon/config.toml
...
[client]
customer_id = "<copy WORKSPACE ID from Azure Portal>"
shared_key = "<copy PRIMARY or SECONDARY KEY from Azure Portal>"
...
```

Restart linux daemon and verify it successfully started.
```bash 
sudo systemctl restart docmon.service
sudo systemctl status docmon.service
```
