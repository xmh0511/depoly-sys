import { Button, Form, Input, message } from "antd";
import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import axios from "axios";
import qs from "qs";
import "./Login.css";
export default function Login() {
  const token = window.localStorage.getItem("token");
  const [userName, setUserName] = useState("");
  const [passWord, setPassWord] = useState("");
  const navigate = useNavigate();
  useEffect(() => {
    const check = async () => {
      try {
        let r = await axios.post(`${window.apiUri}/api/check`, null, {
          headers: {
            Authorization: `Bearer ${token}`,
          },
        });
        //console.log(r);
        if (r.data.status === 200) {
          navigate("/home");
        }
      } catch (e) {
        //console.log("errr",e.response.data.msg);
        //message.error(e.response.data.msg);
      }
    };
    check();
  }, [token, navigate]);
  const onFinish = async () => {
    //console.log(userName,passWord);
    try {
      let r = await axios.post(
        `${window.apiUri}/api/login`,
        qs.stringify({
          name: userName,
          pass: passWord,
        })
      );
      console.log(r);
      if (r.data.status === 200) {
        window.localStorage.setItem("token", r.data.msg.token);
        navigate("/home");
      }
    } catch (e) {
      //console.log("errr",e.response.data.msg);
      message.error(e.response.data.msg);
    }
  };
  return (
    <div className="login-content">
      <div className="login-group">
        <h3 className="title">部署系统后台</h3>
        <Form
          name="basic"
          labelCol={{ span: 8 }}
          wrapperCol={{ span: 16 }}
          style={{ maxWidth: 600 }}
          onFinish={onFinish}
          autoComplete="off"
        >
          <Form.Item
            label="用户名"
            name="用户名"
            rules={[{ required: true, message: "Please input your username!" }]}
          >
            <Input
              value={userName}
              onChange={(e) => {
                setUserName(e.target.value);
              }}
            />
          </Form.Item>

          <Form.Item
            label="密码"
            name="密码"
            rules={[{ required: true, message: "Please input your password!" }]}
          >
            <Input.Password
              value={passWord}
              onChange={(e) => {
                setPassWord(e.target.value);
              }}
            />
          </Form.Item>

          <Form.Item wrapperCol={{ offset: 8, span: 16 }}>
            <Button type="primary" htmlType="submit">
              登录
            </Button>
          </Form.Item>
        </Form>
      </div>
    </div>
  );
}
