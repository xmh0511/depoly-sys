import axios from "axios";
import { useNavigate } from "react-router-dom";
import { useEffect, useState } from "react";
import { Button, Form, Input, message, Table, Modal, Popconfirm } from "antd";
import moment from "moment";
import qs from "qs";
import './App.css';

function App() {
	const token = window.localStorage.getItem("token");
	console.log(token);
	const navigate = useNavigate();
	const [hostList, setHostList] = useState([]);
	const [addhostdialog, setAddhostdialog] = useState(false);
	const [addprojectdialog, setAddprojectdialog] = useState(false);
	const [host, setHost] = useState("");
	const [secret, setSecret] = useState("");
	const [protocol, setProtocol] = useState("http");
	const [name, setName] = useState("");
	const [path, setPath] = useState("");
	const [parent_id, setParent_id] = useState("");
	const [proprotocol, setProprotocol] = useState("http");
	const [editHostDialog, setEditHostDialog] = useState(false);
	const [editHostId, setEditHostId] = useState("");
	const getHostList = async () => {
		try {
			const r = await axios.get(`${window.apiUri}/api/host/list`, {
				headers: {
					"Authorization": `Bearer ${token}`
				}
			});
			console.log(r);
			if (r.data.status === 200) {
				setHostList(r.data.msg.list);
			} else {
				message.error(`${r.data.msg}`);
			}
		} catch (e) {
			if (e.status === 401) {
				navigate("/login");
			}
		}
	}
	useEffect(() => {
		//console.log(protocol);
		getHostList();
	}, []);

	const columns = [
		{ title: 'ID', dataIndex: 'id', key: 'id' },
		{ title: 'host', dataIndex: 'host', key: 'host' },
		{
			title: 'create_time', dataIndex: 'create_time', key: 'create_time', render: (text, record) => {
				return <div>{new moment(text).format("YYYY-MM-DD HH:mm:ss")}</div>
			}
		},
		{
			title: 'update_time', dataIndex: 'update_time', key: 'update_time', render: (text, record) => {
				return <div>{new moment(text).format("YYYY-MM-DD HH:mm:ss")}</div>
			}
		},
		{ title: 'secret', dataIndex: 'secret', key: 'secret' },
		{ title: 'protocol', dataIndex: 'protocol', key: 'protocol' },
		{
			title: '操作',
			dataIndex: 'operation',
			key: 'operation',
			render: (text, record) => {
				return (
					<div>
						<Button type="primary" onClick={() => {
							setParent_id(record.id);
							setProprotocol("http");
							setPath("");
							setName("");
							setAddprojectdialog(true);
						}}>新增</Button>
						<Button onClick={() => {
							setHost(record.host);
							setSecret(record.secret);
							setEditHostId(record.id);
							setEditHostDialog(true);
						}}>编辑</Button>
						<Popconfirm title="是否确认?" onConfirm={async () => {
							//console.log(1111111111, record);
							try {
								let r = await axios.post(`${window.apiUri}/api/host/del`, qs.stringify({
									id: record.id
								}), {
									headers: {
										Authorization: `Bearer ${token}`
									}
								});
								//console.log(r);
								if (r.data.status === 200) {
									getHostList();
								} else {
									message.error(`${r.data.msg}`);
								}
							} catch (e) {
								if (e.status === 401) {
									navigate("/login");
								}
							}
						}}>
							<Button>删除</Button>
						</Popconfirm>
					</div>
				)
			},
		},
	];

	const NestedColumn = [
		{
			title: "id",
			dataIndex: 'id',
			key: 'id'
		},
		{
			title: "name",
			dataIndex: 'name',
			key: 'name'
		},
		{
			title: "path",
			dataIndex: 'path',
			key: 'path'
		},
		{
			title: "token",
			dataIndex: 'token',
			key: 'token'
		},
		{
			title: "create_time",
			dataIndex: 'create_time',
			key: 'create_time',
			render: (text, record) => {
				return <div>{new moment(text).format("YYYY-MM-DD HH:mm:ss")}</div>
			}
		},
		{
			title: "update_time",
			dataIndex: 'update_time',
			key: 'update_time',
			render: (text, record) => {
				return <div>{new moment(text).format("YYYY-MM-DD HH:mm:ss")}</div>
			}
		},
		{
			title: "操作",
			render(text, record) {
				return <div>
					<Popconfirm title="是否确认?" onConfirm={async () => {
						try {
							let r = await axios.post(`${window.apiUri}/api/project/del`, qs.stringify({
								id: record.id
							}), {
								headers: {
									Authorization: `Bearer ${token}`
								}
							});
							//console.log(r);
							if (r.data.status === 200) {
								getHostList();
							} else {
								message.error(`${r.data.msg}`);
							}
						} catch (e) {
							if (e.status === 401) {
								navigate("/login");
							}
						}
					}}>
						<Button>删除</Button>
					</Popconfirm>
				</div>
			}
		}
	]

	return (
		<div className="App">
			<Modal title="编辑主机" open={editHostDialog} onOk={async () => {
				try {
					let r = await axios.post(`${window.apiUri}/api/host/edit`, qs.stringify({
						id: editHostId,
						secret,
						host,
						protocol
					}), {
						headers: {
							Authorization: `Bearer ${token}`
						}
					});
					if (r.data.status === 200) {
						getHostList();
						setEditHostDialog(false);
					} else {
						message.error(`${r.data.msg}`);
					}
				} catch (e) {
					if (e.status === 401) {
						navigate("/login");
					} else {
						//console.log(e);
						message.error(`${e.response.data.msg}`);
					}
				}
			}} onCancel={() => setEditHostDialog(false)}>
				<Form labelCol={{ span: 4 }} initialValues={{ secret, host, protocol }}>
					<Form.Item
						label="host"
						name="host"
						rules={[{ required: true, message: "Please input your host!" }]}
					>
						<Input
							value={host}
							onChange={(e) => {
								setHost(e.target.value);
							}}
						/>
					</Form.Item>
					<Form.Item
						label="secret"
						name="secret"
						rules={[{ required: true, message: "Please input your secret!" }]}
					>
						<Input
							value={secret}
							onChange={(e) => {
								setSecret(e.target.value);
							}}
						/>
					</Form.Item>
					<Form.Item
						label="protocol"
						name="protocol"
						rules={[{ required: true, message: "Please input your protocol!" }]}
					>
						<Input
							value={protocol}
							onChange={(e) => {
								setProtocol(e.target.value);
							}}
						/>
					</Form.Item>
				</Form>
			</Modal>
			<Modal title="新增项目" open={addprojectdialog} onOk={async () => {
				try {
					let r = await axios.post(`${window.apiUri}/api/project/add`, qs.stringify({
						name,
						parent_id,
						path,
						protocol: proprotocol
					}), {
						headers: {
							Authorization: `Bearer ${token}`
						}
					});
					if (r.data.status === 200) {
						getHostList();
						setAddprojectdialog(false);
					} else {
						message.error(`${r.data.msg}`);
					}
				} catch (e) {
					if (e.status === 401) {
						navigate("/login");
					}
				}
			}} onCancel={() => setAddprojectdialog(false)}>
				<Form labelCol={{ span: 4 }} initialValues={{ protocol: proprotocol }}>
					<Form.Item
						label="name"
						name="name"
						rules={[{ required: true, message: "Please input your name!" }]}
					>
						<Input
							value={name}
							onChange={(e) => {
								setName(e.target.value);
							}}
						/>
					</Form.Item>
					<Form.Item
						label="path"
						name="path"
						rules={[{ required: true, message: "Please input your path!" }]}
					>
						<Input
							value={path}
							onChange={(e) => {
								setPath(e.target.value);
							}}
						/>
					</Form.Item>
					<Form.Item
						label="protocol"
						name="protocol"
						rules={[{ required: true, message: "Please input your protocol!" }]}
					>
						<Input
							value={proprotocol}
							onChange={(e) => {
								setProprotocol(e.target.value);
							}}
						/>
					</Form.Item>
				</Form>
			</Modal>
			<Modal title="新增主机" open={addhostdialog} onOk={async () => {
				try {
					let r = await axios.post(`${window.apiUri}/api/host/add`, qs.stringify({
						host,
						secret,
						protocol
					}), {
						headers: {
							Authorization: `Bearer ${token}`
						}
					});
					if (r.data.status === 200) {
						getHostList();
						setAddhostdialog(false);
					} else {
						message.error(`${r.data.msg}`);
					}
				} catch (e) {
					if (e.status === 401) {
						navigate("/login");
					}
				}
			}} onCancel={() => {
				setAddhostdialog(false);
			}}>
				<div>
					<Form labelCol={{ span: 4 }} initialValues={{ protocol }} >
						<Form.Item
							label="host"
							name="host"
							rules={[{ required: true, message: "Please input your host!" }]}
						>
							<Input
								value={host}
								onChange={(e) => {
									setHost(e.target.value);
								}}
							/>
						</Form.Item>
						<Form.Item
							label="secret"
							name="secret"
							rules={[{ required: true, message: "Please input your secret!" }]}
						>
							<Input
								value={secret}
								onChange={(e) => {
									setSecret(e.target.value);
								}}
							/>
						</Form.Item>
						<Form.Item
							label="protocol"
							name="protocol"
							rules={[{ required: true, message: "Please input your protocol!" }]}
						>
							<Input
								value={protocol}
								onChange={(e) => {
									setProtocol(e.target.value);
								}}
							/>
						</Form.Item>
					</Form>
				</div>
			</Modal>
			<div className='header-operation'>
				<Button type='primary' onClick={() => {
					setHost("");
					setSecret("");
					setProtocol("http");
					setAddhostdialog(true);
				}}>新增主机</Button>
			</div>
			<Table
				columns={columns}
				expandable={{
					expandedRowRender: (record) => {
						return (
							<div className="nested-table">
								<Table columns={NestedColumn} dataSource={record.projects} rowKey={(recordNest) => {
									return recordNest.id;
								}} pagination={false}></Table>
							</div>
						);
					},
				}}
				dataSource={hostList}
				rowKey={(record) => {
					return record.id;
				}}
				pagination={false}
			/>
		</div>
	);
}

export default App;
