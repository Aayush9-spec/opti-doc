"""
OptiDock AI — Memory Context Engine

Stores filtered user context in Supabase. It uses a Gemini LLM to filter raw
logs and extract crucial context (system failures, workflow details, etc)
and automatically syncs them to the remote database so they are never lost.
"""

import os
import json
from datetime import datetime
import google.generativeai as genai
from supabase import create_client, Client
from dotenv import load_dotenv

load_dotenv()

# Setup Supabase
SUPABASE_URL = os.environ.get("NEXT_PUBLIC_SUPABASE_URL")
SUPABASE_KEY = os.environ.get("NEXT_PUBLIC_SUPABASE_PUBLISHABLE_KEY")

if SUPABASE_URL and SUPABASE_KEY:
    supabase: Client = create_client(SUPABASE_URL, SUPABASE_KEY)
else:
    supabase = None

# Setup Gemini
GEMINI_KEY = os.environ.get("GEMINI_API_KEY")
if GEMINI_KEY:
    genai.configure(api_key=GEMINI_KEY)
    
def filter_context(raw_text: str) -> dict:
    """
    Uses Gemini to filter raw context and extract crucial details an LLM needs.
    """
    if not GEMINI_KEY:
        return {
            "working_details": "Unfiltered because API key is missing", 
            "system_failures": raw_text,
            "crucial_facts": ""
        }
        
    model = genai.GenerativeModel('gemini-2.5-flash')
    
    prompt = f"""
    You are OptiDock AI's Context Filter. Your job is to extract long-term memory for an AI DevOps agent.
    Filter the following raw terminal output / user input and extract only the crucial data the agent needs to remember.
    
    Format the response as pure JSON matching this exact structure:
    {{
       "working_details": "Description of what the user is working on, repo state, goals",
       "system_failures": "List of errors, tracebacks, or system failure logs",
       "crucial_facts": "Important commands, ports, file paths, or constraints"
    }}
    
    Raw text to filter:
    {raw_text}
    """
    
    try:
        response = model.generate_content(prompt)
        text = response.text.strip()
        # Clean up Markdown formatting from JSON block
        if text.startswith("```json"):
            text = text[7:]
        if text.endswith("```"):
            text = text[:-3]
        return json.loads(text.strip())
    except Exception as e:
        # Fallback if Gemini fails
        return {
            "working_details": "Fallback context extraction",
            "system_failures": raw_text[:500], 
            "crucial_facts": f"Extraction failed: {str(e)}"
        }

def store_memory_to_supabase(user_id: str, raw_context: str):
    """
    Filters context through Gemini and pushes the refined memory to Supabase.
    Table required: `llm_memory` (user_id, working_details, system_failures, crucial_facts, created_at)
    """
    if not supabase:
        print("Warning: Supabase credentials not found. Memory will not be stored.")
        return None
        
    filtered_data = filter_context(raw_context)
    
    entry = {
        "user_id": user_id,
        "working_details": filtered_data.get("working_details", ""),
        "system_failures": filtered_data.get("system_failures", ""),
        "crucial_facts": filtered_data.get("crucial_facts", ""),
        "created_at": datetime.now().isoformat()
    }
    
    try:
        response = supabase.table("llm_memory").insert(entry).execute()
        return response.data
    except Exception as e:
        print(f"Error saving to Supabase: {e}")
        print("Make sure a table named 'llm_memory' exists with columns: user_id, working_details, system_failures, crucial_facts!")
        return None
        
if __name__ == "__main__":
    # Test execution
    test_log = "Run cargo check failed. Error[E0428]: name render_provider_report is defined multiple times. Found previous definition at line 830."
    print("Testing Filter:")
    print(json.dumps(filter_context(test_log), indent=2))
