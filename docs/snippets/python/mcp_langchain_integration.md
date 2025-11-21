```python
from langchain.agents import initialize_agent, AgentType
from langchain.tools import Tool
from langchain_openai import ChatOpenAI
import subprocess
import json

# Start MCP server
mcp_process = subprocess.Popen(
    ["kreuzberg", "mcp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

def extract_file(path: str) -> str:
    request = {
        "method": "tools/call",
        "params": {
            "name": "extract_file",
            "arguments": {"path": path, "async": True}
        }
    }
    mcp_process.stdin.write(json.dumps(request).encode() + b"\n")
    mcp_process.stdin.flush()
    response = mcp_process.stdout.readline()
    return json.loads(response)["result"]["content"]

tools = [
    Tool(
        name="extract_document",
        func=extract_file,
        description="Extract text from documents (PDF, DOCX, images, etc.)"
    )
]

llm = ChatOpenAI(temperature=0)
agent = initialize_agent(
    tools, llm, agent=AgentType.ZERO_SHOT_REACT_DESCRIPTION, verbose=True
)

agent.run("Extract the content from contract.pdf and summarize it")
```
