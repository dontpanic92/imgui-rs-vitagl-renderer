void ImGui_ImplVitaGL_InitTouch(void);
void ImGui_ImplVitaGL_PollTouch(double x0, double y0, double sx, double sy, int *mx, int *my, bool *mbuttons);

extern "C"
{
    void c_ImGui_ImplVitaGL_InitTouch(void) { ImGui_ImplVitaGL_InitTouch(); }
    void c_ImGui_ImplVitaGL_PollTouch(double x0, double y0, double sx, double sy, int *mx, int *my, bool *mbuttons)
    {
        ImGui_ImplVitaGL_PollTouch(x0, y0, sx, sy, mx, my, mbuttons);
    }
}