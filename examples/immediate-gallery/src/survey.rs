use super::*;
#[derive(Clone, Debug, PartialEq)]
pub struct SurveyResult {
    pub role: String,
    pub tech_stack: Vec<String>,
    pub feedback: String,
}
impl Gallery {
    pub(super) fn survey(&mut self, p: &mut Painter, step: usize, rect: Rect) {
        let x = rect.x;
        let y = rect.y;
        let titles = [
            "Welcome to the Dynamic Form Demo",
            "What is your primary role?",
            "Which languages or frameworks do you use daily?",
            "How can we make Sensorial UI Kit better for you?",
        ];
        p.label(
            x + 24.0,
            y + 20.0,
            450.0,
            52.0,
            titles[step],
            20.0,
            600,
            p.palette.fg,
        );
        let count = if self.survey_role == Some(0) {
            4.0
        } else {
            3.0
        };
        let position = if step == 3 { count } else { step as f32 + 1.0 };
        p.progress(r(x + 24.0, y + 80.0, 492.0, 5.0), position / count, BLUE);
        match step {
            0=>p.label(x+24.0,y+108.0,492.0,90.0,"Experience intuitive step-by-step interactive forms with logical conditional branching.",16.0,400,p.palette.muted),
            1=>{p.label(x+24.0,y+94.0,492.0,32.0,"We customize your onboarding flow according to your background.",13.0,400,p.palette.muted);for(i,label)in["Software Engineer / Developer","UI/UX Designer","Product Manager"].iter().enumerate(){if p.button(&format!("survey-role-{i}"),r(x+24.0,y+138.0+i as f32*46.0,492.0,38.0),label,if self.survey_role==Some(i){5}else{2},false){self.survey_role=Some(i);}}},
            2=>{p.label(x+24.0,y+102.0,492.0,28.0,"Select all that apply.",13.0,400,p.palette.muted);for(i,label)in["Rust & Dioxus","TypeScript & React","Python & AI Stack"].iter().enumerate(){p.check(&format!("survey-stack-{i}"),r(x+24.0,y+144.0+i as f32*44.0,492.0,36.0),label,&mut self.survey_stack[i],false,false);}},
            _=>{p.label(x+24.0,y+96.0,492.0,48.0,"Tell us about features, components, or improvements you'd like to see.",13.0,400,p.palette.muted);p.field("survey-feedback",r(x+24.0,y+152.0,492.0,126.0),&mut self.survey_feedback,"Type your thoughts and suggestions here...",false);},
        }
        if step > 0
            && p.button(
                "survey-back",
                r(x + 24.0, y + 314.0, 110.0, 38.0),
                "Back",
                2,
                false,
            )
        {
            self.overlay = Some(Overlay::Survey(
                if step == 3 && self.survey_role != Some(0) {
                    1
                } else {
                    step - 1
                },
            ));
        }
        if p.button(
            "survey-next",
            r(x + 366.0, y + 314.0, 150.0, 38.0),
            if step == 3 { "Submit" } else { "Continue" },
            0,
            step == 1 && self.survey_role.is_none(),
        ) {
            if step == 3 {
                let role = self.survey_role.unwrap();
                self.survey_result = Some(SurveyResult {
                    role: ["developer", "designer", "pm"][role].into(),
                    tech_stack: if role == 0 {
                        ["rust", "ts", "python"]
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| self.survey_stack[*i])
                            .map(|(_, s)| (*s).into())
                            .collect()
                    } else {
                        Vec::new()
                    },
                    feedback: self.survey_feedback.clone(),
                });
                self.overlay = None;
            } else {
                self.overlay = Some(Overlay::Survey(
                    if step == 1 && self.survey_role != Some(0) {
                        3
                    } else {
                        step + 1
                    },
                ));
            }
        }
    }
}
